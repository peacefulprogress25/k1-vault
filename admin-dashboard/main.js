import { Connection, PublicKey } from 'https://esm.sh/@solana/web3.js@1.95.3';
import * as anchor from 'https://esm.sh/@coral-xyz/anchor@0.29.0';

const logEl = document.getElementById('log');
const walletStatus = document.getElementById('walletStatus');
const connectBtn = document.getElementById('connectBtn');
const loadProgramBtn = document.getElementById('loadProgramBtn');
const idlFile = document.getElementById('idlFile');

let provider = null;
let program = null;

const log = (m) => (logEl.textContent = `[${new Date().toISOString()}] ${m}\n` + logEl.textContent);
const parseU64 = (s) => new anchor.BN(s || '0');
const toPk = (s) => new PublicKey(s);
const hexToBytes32 = (hex) => {
  if (hex.length !== 64) throw new Error('oracleFeedId must be 64 hex chars');
  return Array.from(new Uint8Array(hex.match(/.{1,2}/g).map((b) => parseInt(b, 16))));
};
const parseAccountsJson = (txt) => JSON.parse(txt || '{}');
const parseRemainingJson = (txt) => (txt ? JSON.parse(txt).map((k) => ({ pubkey: toPk(k), isSigner: false, isWritable: true })) : []);

connectBtn.onclick = async () => {
  if (!window.solana?.isPhantom) return log('Phantom wallet not found');
  await window.solana.connect();
  walletStatus.textContent = `Wallet: ${window.solana.publicKey.toBase58()}`;
  log('Wallet connected');
};

loadProgramBtn.onclick = async () => {
  const file = idlFile.files?.[0];
  if (!file) return log('Please upload generated IDL JSON first');
  const idl = JSON.parse(await file.text());
  const rpc = document.getElementById('rpcUrl').value.trim();
  const pid = document.getElementById('programId').value.trim();
  const connection = new Connection(rpc, 'confirmed');
  provider = new anchor.AnchorProvider(connection, window.solana, { commitment: 'confirmed' });
  anchor.setProvider(provider);
  program = new anchor.Program(idl, new PublicKey(pid), provider);
  log('Program client loaded');
};

async function send(methodName, args, accounts, remainingAccounts = []) {
  if (!program) throw new Error('Load IDL/program first');
  const tx = await program.methods[methodName](...args).accounts(accounts).remainingAccounts(remainingAccounts).rpc();
  log(`${methodName} sent: ${tx}`);
}

document.querySelectorAll('form').forEach((form) => {
  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    try {
      const fd = new FormData(form);
      const vaultState = toPk(document.getElementById('vaultState').value.trim());
      const signer = window.solana.publicKey;
      const method = form.dataset.method;

      if (method === 'depositUser') {
        const accounts = parseAccountsJson(fd.get('accountsJson'));
        const mapped = Object.fromEntries(Object.entries(accounts).map(([k, v]) => [k, toPk(v)]));
        await send('deposit', [parseU64(fd.get('maxAmount'))], mapped, parseRemainingJson(fd.get('remainingJson')));
      } else if (method === 'withdrawUser') {
        const accounts = parseAccountsJson(fd.get('accountsJson'));
        const mapNode = (obj) => Object.fromEntries(Object.entries(obj).map(([k, v]) => [k, typeof v === 'object' ? mapNode(v) : toPk(v)]));
        await send('withdraw', [parseU64(fd.get('sharesAmount'))], mapNode(accounts), parseRemainingJson(fd.get('remainingJson')));
      } else if (method === 'withdrawFromAvailableUser') {
        const accounts = parseAccountsJson(fd.get('accountsJson'));
        const mapped = Object.fromEntries(Object.entries(accounts).map(([k, v]) => [k, toPk(v)]));
        await send('withdrawFromAvailable', [parseU64(fd.get('sharesAmount'))], mapped, parseRemainingJson(fd.get('remainingJson')));
      } else if (method === 'addStrategy') {
        await send('addStrategy', [toPk(fd.get('strategyId')), Number(fd.get('strategyType')), parseU64(fd.get('weight')), parseU64(fd.get('cap'))], { signer, vaultState });
      } else if (method === 'updateStrategy') {
        await send('updateStrategy', [toPk(fd.get('strategyId')), parseU64(fd.get('newWeight')), parseU64(fd.get('newCap'))], { signer, vaultState });
      } else if (method === 'removeStrategy') {
        await send('removeStrategy', [toPk(fd.get('strategyId'))], { signer, vaultState });
      } else if (method === 'updateStrategyOracle') {
        await send('updateStrategyOracle', [toPk(fd.get('strategyId')), toPk(fd.get('oraclePriceFeed')), hexToBytes32(fd.get('oracleFeedId')), Number(fd.get('tokenDecimals')), Number(fd.get('withdrawPriority'))], { signer, vaultState });
      } else if (method === 'rebalanceVault') {
        await send('rebalanceVault', [], { signer, vaultState });
      } else if (method === 'refreshStrategyNav') {
        await send('refreshStrategyNav', [toPk(fd.get('strategyId')), parseU64(fd.get('strategyTokenBalance')), parseU64(fd.get('maxPriceAgeSec'))], { signer, vaultState, priceUpdate: toPk(fd.get('priceUpdate')) });
      } else if (method === 'updateAdmin') {
        await send('updateAdmin', [], { adminAuthority: signer, vaultState });
      } else if (method === 'withdrawPendingFees') {
        log('withdraw_pending_fees requires extended account JSON support (pending).');
      } else if (method === 'giveUpPendingFees') {
        await send('giveUpPendingFees', [parseU64(fd.get('maxAmountToGiveUp'))], { signer, vaultState });
      }
    } catch (err) {
      log(`Error: ${err.message || err}`);
    }
  });
});

log('Dashboard ready. Upload IDL, connect wallet, then submit forms.');
