// The bridge is part of the verified local page: no asynchronous script injection
// has to win a race against the first navigation.
window.external = {invoke: message => window.chrome.webview.postMessage(message)};
window._rpc = window._rpc || {nextSeq: 1};
for (const name of ['nativeCall', 'nativeReady', 'nativeLog']) {
  window[name] = (...params) => new Promise((resolve, reject) => {
    const id = window._rpc.nextSeq++;
    window._rpc[id] = {resolve, reject};
    try { window.external.invoke(JSON.stringify({id, method: name, params})); }
    catch (error) { delete window._rpc[id]; reject(error); }
  });
}
