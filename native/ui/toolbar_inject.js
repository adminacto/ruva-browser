(function(){
  if(document.getElementById('ruva-root')) return;
  if(!window.ipc) window.ipc={postMessage:function(){}};

  var root=document.createElement('div');
  root.id='ruva-root';
  root.style.cssText='all:initial !important;position:fixed !important;top:0 !important;left:0 !important;right:0 !important;z-index:2147483647 !important;display:block !important;margin:0 !important;padding:0 !important;border:0 !important;background:none !important;pointer-events:auto !important;';
  var shadow=root.attachShadow({mode:'open'});

  shadow.innerHTML=`
<style>
*{margin:0;padding:0;box-sizing:border-box}
:host{all:initial}
.bar{
  display:flex;align-items:center;gap:4px;
  height:36px;background:#1e1e1e;
  border-bottom:1px solid #333;
  padding:0 6px;
  font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;
  user-select:none;-webkit-app-region:drag;
}
.bar *{-webkit-app-region:no-drag}
.btn{
  background:transparent;border:1px solid transparent;
  color:#999;padding:0 6px;border-radius:6px;
  cursor:pointer;font-size:13px;line-height:1;
  min-width:26px;height:26px;
  display:flex;align-items:center;justify-content:center;
  transition:background .15s,color .15s;
}
.btn:hover{background:#333;color:#ddd}
.btn:active{background:#444}
.btn.nav{font-size:15px}
.url-bar{
  flex:1;background:#2a2a2a;border:1px solid #444;
  color:#ddd;padding:0 10px;border-radius:12px;
  font-size:12px;height:26px;outline:none;
  transition:border-color .2s;
}
.url-bar:focus{border-color:#4a90d9}
.url-bar::placeholder{color:#666}
.sep{width:1px;height:18px;background:#333;flex-shrink:0;margin:0 2px}
.tabs{
  display:flex;gap:3px;align-items:center;
  max-width:260px;overflow-x:auto;flex-shrink:0;
  scrollbar-width:none;
}
.tabs::-webkit-scrollbar{display:none}
.tab{
  background:#2a2a2a;border:1px solid #333;
  color:#888;padding:0 8px;border-radius:6px;
  font-size:11px;cursor:pointer;
  white-space:nowrap;max-width:110px;overflow:hidden;
  text-overflow:ellipsis;height:24px;
  display:flex;align-items:center;gap:4px;
  transition:background .15s,color .15s;
  flex-shrink:0;
}
.tab:hover{background:#333;color:#bbb}
.tab.active{background:#3b82f6;color:#fff;border-color:#3b82f6}
.tab .close{
  font-size:10px;opacity:.4;cursor:pointer;
  width:14px;height:14px;display:flex;
  align-items:center;justify-content:center;
  border-radius:3px;flex-shrink:0;
}
.tab .close:hover{opacity:1;background:rgba(255,255,255,.15)}
.tab .title{overflow:hidden;text-overflow:ellipsis}
</style>

<div class="bar">
  <button class="btn nav" id="btnNew" title="New Tab">+</button>
  <button class="btn nav" id="btnBack" title="Back">\u2190</button>
  <button class="btn nav" id="btnFwd" title="Forward">\u2192</button>
  <button class="btn nav" id="btnReload" title="Reload">\u27F3</button>
  <button class="btn nav" id="btnHome" title="Home">\u2302</button>
  <div class="sep"></div>
  <div class="tabs" id="tabBar"></div>
  <div class="sep"></div>
  <input class="url-bar" id="urlBar" placeholder="Enter URL or search...">
  <button class="btn nav" id="btnSettings" title="Settings">\u2630</button>
</div>`;

  document.documentElement.appendChild(root);
  document.body.style.marginTop='37px';
  document.body.style.paddingTop='0';

  var s=shadow;
  function post(msg){window.ipc.postMessage(JSON.stringify(msg))}

  s.getElementById('btnNew').onclick=function(){post({cmd:'new_tab'})};
  s.getElementById('btnBack').onclick=function(){post({cmd:'back'})};
  s.getElementById('btnFwd').onclick=function(){post({cmd:'forward'})};
  s.getElementById('btnReload').onclick=function(){post({cmd:'reload'})};
  s.getElementById('btnHome').onclick=function(){post({cmd:'navigate',url:''})};
  s.getElementById('btnSettings').onclick=function(){post({cmd:'open_settings'})};

  var urlBar=s.getElementById('urlBar');
  urlBar.value=location.href;
  urlBar.onkeydown=function(e){
    if(e.key==='Enter'){post({cmd:'navigate',url:urlBar.value});urlBar.blur();}
  };

  window.__ruvaUpdateUrl=function(u){urlBar.value=u||location.href};

  var tabsData=[];
  window.__ruvaUpdateTabs=function(tabs,activeId){
    tabsData=tabs||[];
    var bar=s.getElementById('tabBar');
    bar.innerHTML='';
    tabsData.forEach(function(t){
      var el=document.createElement('div');
      el.className='tab'+(t.id===activeId?' active':'');
      el.innerHTML='<span class="title">'+(t.title||'New Tab')+'</span><span class="close" data-id="'+t.id+'">\u2715</span>';
      el.onclick=function(e){
        if(e.target.classList.contains('close')){
          post({cmd:'close_tab',tab_id:e.target.getAttribute('data-id')});
        }else{
          post({cmd:'switch_tab',tab_id:t.id});
        }
      };
      bar.appendChild(el);
    });
    if(tabsData.length<=1){bar.style.display='none';s.querySelector('.sep').style.display='none';}
    else{bar.style.display='flex';s.querySelector('.sep').style.display='block';}
  };
})();
