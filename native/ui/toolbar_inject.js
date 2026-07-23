(function(){
  if(document.getElementById('ruva-toolbar')) return;
  var bar=document.createElement('div');
  bar.id='ruva-toolbar';
  bar.style.cssText='position:fixed;top:0;left:0;right:0;height:40px;background:#2b2b2b;border-bottom:1px solid #444;z-index:2147483647;display:flex;align-items:center;padding:0 8px;gap:6px;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;';
  bar.innerHTML=`
<style>
#ruva-toolbar button,#ruva-toolbar input{-webkit-app-region:no-drag;}
#ruva-toolbar .btn{background:#3b3b3b;border:1px solid #555;color:#ccc;padding:4px 10px;border-radius:6px;cursor:pointer;font-size:14px;line-height:1;min-width:28px;height:28px;display:flex;align-items:center;justify-content:center;}
#ruva-toolbar .btn:hover{background:#4a4a4a;border-color:#666;}
#ruva-toolbar .url-bar{flex:1;background:#1a1a1a;border:1px solid #555;color:#e0e0e0;padding:4px 12px;border-radius:14px;font-size:13px;height:28px;outline:none;}
#ruva-toolbar .url-bar:focus{border-color:#4a90d9;}
</style>
<button class="btn" onclick="window.ipc.postMessage(JSON.stringify({cmd:'navigate',url:''}))" title="New Tab">+</button>
<button class="btn" onclick="window.ipc.postMessage(JSON.stringify({cmd:'back'}))" title="Back">\u2190</button>
<button class="btn" onclick="window.ipc.postMessage(JSON.stringify({cmd:'forward'}))" title="Forward">\u2192</button>
<button class="btn" onclick="window.ipc.postMessage(JSON.stringify({cmd:'reload'}))" title="Reload">\u27F3</button>
<button class="btn" onclick="window.ipc.postMessage(JSON.stringify({cmd:'navigate',url:''}))" title="Home">\u2302</button>
<input class="url-bar" id="ruvaUrlBar" placeholder="Enter URL or search...">
<button class="btn" onclick="window.ipc.postMessage(JSON.stringify({cmd:'open_settings'}))" title="Settings">\u2630</button>`;
  document.body.prepend(bar);
  var u=document.getElementById('ruvaUrlBar');
  if(u){u.value=location.href;
    u.onkeydown=function(e){if(e.key==='Enter'){window.ipc.postMessage(JSON.stringify({cmd:'navigate',url:u.value}));u.blur();}};
  }
  document.body.style.marginTop='40px';
  document.body.style.paddingTop='0';
})();
