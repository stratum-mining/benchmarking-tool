"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const rest_1 = require("./rest");
class RPCClient extends rest_1.RESTClient {
  constructor({ user = "", pass, wallet, fullResponse, ...options }) {
    super({ ...options, auth: { user, pass }, uri: "/" });
    this.fullResponse = fullResponse ? true : false;
    this.wallet = typeof wallet === "string" ? wallet : undefined;
  }
  batch(body, uri = "/") {
    return super.post({ body, uri });
  }
  async rpc(method, params = {}, wallet) {
    const uri = typeof wallet === "undefined" ? "/" : "wallet/" + wallet;
    const body = { method, params, jsonrpc: "1.0", id: "rpc-bitcoin" };
    try {
      const response = await this.batch(body, uri);
      return this.fullResponse ? response : response.result;
    } catch (error) {
      if (error.error && error.error.error && error.error.result === null) {
        throw this.fullResponse ? error.error : error.error.error;
      }
      throw error;
    }
  }
  getbestblockhash() {
    return this.rpc("getbestblockhash");
  }
  getblock({ blockhash, verbosity = 1 }) {
    return this.rpc("getblock", { blockhash, verbosity });
  }
  getblockchaininfo() {
    return this.rpc("getblockchaininfo");
  }
  getblockcount() {
    return this.rpc("getblockcount");
  }
  getblockfilter(options) {
    return this.rpc("getblockfilter", options);
  }
  getblockhash({ height }) {
    return this.rpc("getblockhash", { height });
  }
  getblockheader({ blockhash, verbose = true }) {
    return this.rpc("getblockheader", { blockhash, verbose });
  }
  getblockstats({ hash_or_height, stats = [] }) {
    return this.rpc("getblockstats", { hash_or_height, stats });
  }
  getchaintips() {
    return this.rpc("getchaintips");
  }
  getchaintxstats({ nblocks, blockhash }) {
    return this.rpc("getchaintxstats", { nblocks, blockhash });
  }
  getdifficulty() {
    return this.rpc("getdifficulty");
  }
  getmempoolancestors({ txid, verbose = false }) {
    return this.rpc("getmempoolancestors", { txid, verbose });
  }
  getmempooldescendants({ txid, verbose = false }) {
    return this.rpc("getmempooldescendants", { txid, verbose });
  }
  getmempoolentry({ txid }) {
    return this.rpc("getmempoolentry", { txid });
  }
  getmempoolinfo() {
    return this.rpc("getmempoolinfo");
  }
  getrawmempool({ verbose = false } = {}) {
    return this.rpc("getrawmempool", { verbose });
  }
  gettxout({ txid, n, include_mempool = true }) {
    return this.rpc("gettxout", { txid, n, include_mempool });
  }
  gettxoutproof({ txids, blockhash }) {
    return this.rpc("gettxoutproof", { txids, blockhash });
  }
  gettxoutsetinfo() {
    return this.rpc("gettxoutsetinfo");
  }
  preciousblock({ blockhash }) {
    return this.rpc("preciousblock", { blockhash });
  }
  pruneblockchain({ height }) {
    return this.rpc("pruneblockchain", { height });
  }
  savemempool() {
    return this.rpc("savemempool");
  }
  scantxoutset({ action, scanobjects }) {
    return this.rpc("scantxoutset", { action, scanobjects });
  }
  verifychain({ checklevel = 3, nblocks = 6 } = {}) {
    return this.rpc("verifychain", { checklevel, nblocks });
  }
  verifytxoutproof({ proof }) {
    return this.rpc("verifytxoutproof", { proof });
  }
  getmemoryinfo({ mode = "stats" } = {}) {
    return this.rpc("getmemoryinfo", { mode });
  }
  getrpcinfo() {
    return this.rpc("getrpcinfo");
  }
  help({ command } = {}) {
    return this.rpc("help", { command });
  }
  logging({ include, exclude } = {}) {
    return this.rpc("logging", { include, exclude });
  }
  stop() {
    return this.rpc("stop");
  }
  uptime() {
    return this.rpc("uptime");
  }
  generatetoaddress(options, wallet) {
    return this.rpc("generatetoaddress", options, wallet || this.wallet);
  }
  getblocktemplate(options) {
    return this.rpc("getblocktemplate", options);
  }
  getmininginfo() {
    return this.rpc("getmininginfo");
  }
  getnetworkhashps(options = {}) {
    return this.rpc("getnetworkhashps", options);
  }
  prioritisetransaction(options) {
    return this.rpc("prioritisetransaction", options);
  }
  submitblock(options) {
    return this.rpc("submitblock", options);
  }
  submitheader(options) {
    return this.rpc("submitheader", options);
  }
  addnode(options) {
    return this.rpc("addnode", options);
  }
  clearbanned() {
    return this.rpc("clearbanned");
  }
  disconnectnode(params) {
    if ("address" in params) {
      return this.rpc("disconnectnode", { address: params.address });
    }
    return this.rpc("disconnectnode", { nodeid: params.nodeid });
  }
  getaddednodeinfo(options = {}) {
    return this.rpc("getaddednodeinfo", options);
  }
  getconnectioncount() {
    return this.rpc("getconnectioncount");
  }
  getnettotals() {
    return this.rpc("getnettotals");
  }
  getnetworkinfo() {
    return this.rpc("getnetworkinfo");
  }
  getnodeaddresses(options = {}) {
    return this.rpc("getnodeaddresses", options);
  }
  getpeerinfo() {
    return this.rpc("getpeerinfo");
  }
  listbanned() {
    return this.rpc("listbanned");
  }
  ping() {
    return this.rpc("ping");
  }
  setban(options) {
    return this.rpc("setban", options);
  }
  setnetworkactive(options) {
    return this.rpc("setnetworkactive", options);
  }
  analyzepsbt(options) {
    return this.rpc("analyzepsbt", options);
  }
  combinepsbt(options) {
    return this.rpc("combinepsbt", options);
  }
  combinerawtransaction(options) {
    return this.rpc("combinerawtransaction", options);
  }
  converttopsbt(options) {
    return this.rpc("converttopsbt", options);
  }
  createpsbt(options) {
    return this.rpc("createpsbt", options);
  }
  createrawtransaction(options) {
    return this.rpc("createrawtransaction", options);
  }
  decodepsbt(options) {
    return this.rpc("decodepsbt", options);
  }
  decoderawtransaction(options) {
    return this.rpc("decoderawtransaction", options);
  }
  decodescript(options) {
    return this.rpc("decodescript", options);
  }
  finalizepsbt(options) {
    return this.rpc("finalizepsbt", options);
  }
  fundrawtransaction(options, wallet) {
    return this.rpc("fundrawtransaction", options, wallet || this.wallet);
  }
  getrawtransaction(options) {
    return this.rpc("getrawtransaction", options);
  }
  joinpsbts(options) {
    return this.rpc("joinpsbts", options);
  }
  sendrawtransaction(options) {
    return this.rpc("sendrawtransaction", options);
  }
  signrawtransactionwithkey(options) {
    return this.rpc("signrawtransactionwithkey", options);
  }
  testmempoolaccept(options) {
    return this.rpc("testmempoolaccept", options);
  }
  utxoupdatepsbt(options) {
    return this.rpc("utxoupdatepsbt", options);
  }
  createmultisig(options) {
    return this.rpc("createmultisig", options);
  }
  deriveaddresses({ descriptor, range }) {
    return this.rpc("deriveaddresses", { descriptor, range });
  }
  estimatesmartfee(options) {
    return this.rpc("estimatesmartfee", options);
  }
  getdescriptorinfo(options) {
    return this.rpc("getdescriptorinfo", options);
  }
  signmessagewithprivkey(options) {
    return this.rpc("signmessagewithprivkey", options);
  }
  validateaddress(options) {
    return this.rpc("validateaddress", options);
  }
  verifymessage(options) {
    return this.rpc("verifymessage", options);
  }
  abandontransaction(options, wallet) {
    return this.rpc("abandontransaction", options, wallet || this.wallet);
  }
  abortrescan(wallet) {
    return this.rpc("abortrescan", undefined, wallet || this.wallet);
  }
  addmultisigaddress(options, wallet) {
    return this.rpc("addmultisigaddress", options, wallet || this.wallet);
  }
  backupwallet(options, wallet) {
    return this.rpc("backupwallet", options, wallet || this.wallet);
  }
  bumpfee(options, wallet) {
    return this.rpc("bumpfee", options, wallet || this.wallet);
  }
  createwallet(options) {
    return this.rpc("createwallet", options);
  }
  dumpprivkey(options, wallet) {
    return this.rpc("dumpprivkey", options, wallet || this.wallet);
  }
  dumpwallet(options, wallet) {
    return this.rpc("dumpwallet", options, wallet || this.wallet);
  }
  encryptwallet(options, wallet) {
    return this.rpc("encryptwallet", options, wallet || this.wallet);
  }
  getaddressesbylabel(options, wallet) {
    return this.rpc("getaddressesbylabel", options, wallet || this.wallet);
  }
  getaddressinfo(options, wallet) {
    return this.rpc("getaddressinfo", options, wallet || this.wallet);
  }
  getbalance(options, wallet) {
    return this.rpc("getbalance", options, wallet || this.wallet);
  }
  getbalances(wallet) {
    return this.rpc("getbalances", undefined, wallet || this.wallet);
  }
  getnewaddress(options, wallet) {
    return this.rpc("getnewaddress", options, wallet || this.wallet);
  }
  getrawchangeaddress(options, wallet) {
    return this.rpc("getrawchangeaddress", options, wallet || this.wallet);
  }
  getreceivedbyaddress(options, wallet) {
    return this.rpc("getreceivedbyaddress", options, wallet || this.wallet);
  }
  getreceivedbylabel(options, wallet) {
    return this.rpc("getreceivedbylabel", options, wallet || this.wallet);
  }
  gettransaction(options, wallet) {
    return this.rpc("gettransaction", options, wallet || this.wallet);
  }
  getunconfirmedbalance(wallet) {
    return this.rpc("getunconfirmedbalance", undefined, wallet || this.wallet);
  }
  getwalletinfo(wallet) {
    return this.rpc("getwalletinfo", undefined, wallet || this.wallet);
  }
  importaddress(options, wallet) {
    return this.rpc("importaddress", options, wallet || this.wallet);
  }
  importmulti(options, wallet) {
    return this.rpc("importmulti", options, wallet || this.wallet);
  }
  importprivkey(options, wallet) {
    return this.rpc("importprivkey", options, wallet || this.wallet);
  }
  importprunedfunds(options, wallet) {
    return this.rpc("importprunedfunds", options, wallet || this.wallet);
  }
  importpubkey(options, wallet) {
    return this.rpc("importpubkey", options, wallet || this.wallet);
  }
  importwallet(options, wallet) {
    return this.rpc("importwallet", options, wallet || this.wallet);
  }
  keypoolrefill(options, wallet) {
    return this.rpc("keypoolrefill", options, wallet || this.wallet);
  }
  listaddressgroupings(wallet) {
    return this.rpc("listaddressgroupings", undefined, wallet || this.wallet);
  }
  listlabels(options, wallet) {
    return this.rpc("listlabels", options, wallet || this.wallet);
  }
  listlockunspent(wallet) {
    return this.rpc("listlockunspent", undefined, wallet || this.wallet);
  }
  listreceivedbyaddress(options, wallet) {
    return this.rpc("listreceivedbyaddress", options, wallet || this.wallet);
  }
  listreceivedbylabel(options, wallet) {
    return this.rpc("listreceivedbylabel", options, wallet || this.wallet);
  }
  listsinceblock(options, wallet) {
    return this.rpc("listsinceblock", options, wallet || this.wallet);
  }
  listtransactions(options, wallet) {
    return this.rpc("listtransactions", options, wallet || this.wallet);
  }
  listunspent(options, wallet) {
    return this.rpc("listunspent", options, wallet || this.wallet);
  }
  listwalletdir() {
    return this.rpc("listwalletdir");
  }
  listwallets() {
    return this.rpc("listwallets");
  }
  loadwallet({ filename }) {
    return this.rpc("loadwallet", { filename });
  }
  lockunspent(options, wallet) {
    return this.rpc("lockunspent", options, wallet || this.wallet);
  }
  removeprunedfunds(options, wallet) {
    return this.rpc("removeprunedfunds", options, wallet || this.wallet);
  }
  rescanblockchain(options, wallet) {
    return this.rpc("rescanblockchain", options, wallet || this.wallet);
  }
  sendmany(options, wallet) {
    return this.rpc("sendmany", options, wallet || this.wallet);
  }
  sendtoaddress(options, wallet) {
    return this.rpc("sendtoaddress", options, wallet || this.wallet);
  }
  sethdseed(options, wallet) {
    return this.rpc("sethdseed", options, wallet || this.wallet);
  }
  setlabel(options, wallet) {
    return this.rpc("setlabel", options, wallet || this.wallet);
  }
  settxfee(options, wallet) {
    return this.rpc("settxfee", options, wallet || this.wallet);
  }
  setwalletflag(options, wallet) {
    return this.rpc("setwalletflag", options, wallet || this.wallet);
  }
  signmessage(options, wallet) {
    return this.rpc("signmessage", options, wallet || this.wallet);
  }
  signrawtransactionwithwallet(options, wallet) {
    return this.rpc(
      "signrawtransactionwithwallet",
      options,
      wallet || this.wallet
    );
  }
  unloadwallet({ wallet_name } = {}) {
    if (typeof wallet_name !== "undefined") {
      return this.rpc("unloadwallet", { wallet_name });
    }
    return this.rpc("unloadwallet", undefined, this.wallet);
  }
  walletcreatefundedpsbt(options, wallet) {
    return this.rpc("walletcreatefundedpsbt", options, wallet || this.wallet);
  }
  walletlock(wallet) {
    return this.rpc("walletlock", undefined, wallet || this.wallet);
  }
  walletpassphrase(options, wallet) {
    return this.rpc("walletpassphrase", options, wallet || this.wallet);
  }
  walletpassphrasechange(options, wallet) {
    return this.rpc("walletpassphrasechange", options, wallet || this.wallet);
  }
  walletprocesspsbt(options, wallet) {
    return this.rpc("walletprocesspsbt", options, wallet || this.wallet);
  }
  getzmqnotifications() {
    return this.rpc("getzmqnotifications");
  }
}
exports.RPCClient = RPCClient;