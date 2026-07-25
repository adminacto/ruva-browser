(function(){
  if(document.getElementById('ruva-toolbar')) return;
  if(!document.body) return;

  function post(obj){
    var m=JSON.stringify(obj);
    try{
      if(window.ipc&&window.ipc.postMessage){window.ipc.postMessage(m);return;}
      if(window.chrome&&window.chrome.webview&&window.chrome.webview.postMessage){window.chrome.webview.postMessage(m);return;}
      if(window.webkit&&window.webkit.messageHandlers&&window.webkit.messageHandlers.ipc){window.webkit.messageHandlers.ipc.postMessage(m);return;}
    }catch(e){}
  }

  var NAV_H=40, TABS_H=34;
  var tabsVisible=true;

  var wrap=document.createElement('div');
  wrap.id='ruva-toolbar';
  wrap.style.cssText='position:fixed;top:0;left:0;right:0;z-index:2147483647;background:#202124;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;box-shadow:0 1px 0 #3c4043;';

  var style=document.createElement('style');
  style.textContent=[
    '#ruva-toolbar *{box-sizing:border-box;margin:0;padding:0;}',
    '#ruva-tabbar{display:flex;align-items:flex-end;height:'+TABS_H+'px;padding:4px 6px 0;gap:1px;overflow-x:auto;overflow-y:hidden;scrollbar-width:none;background:#202124;}',
    '#ruva-tabbar::-webkit-scrollbar{display:none;}',
    '.ruva-tab{display:flex;align-items:center;gap:6px;min-width:100px;max-width:200px;flex:0 1 200px;height:30px;padding:0 8px 0 12px;border-radius:8px 8px 0 0;background:transparent;color:#9aa0a6;font-size:12px;cursor:pointer;user-select:none;white-space:nowrap;}',
    '.ruva-tab:hover{background:#292b2e;color:#bdc1c6;}',
    '.ruva-tab.active{background:#35363a;color:#e8eaed;}',
    '.ruva-tab-title{flex:1;overflow:hidden;text-overflow:ellipsis;}',
    '.ruva-tab-close{width:16px;height:16px;border-radius:50%;border:none;background:transparent;color:inherit;font-size:11px;line-height:16px;text-align:center;cursor:pointer;flex:none;}',
    '.ruva-tab-close:hover{background:rgba(255,255,255,0.15);}',
    '.ruva-newtab{width:26px;height:26px;margin:0 0 2px 4px;border:none;border-radius:50%;background:transparent;color:#9aa0a6;font-size:16px;cursor:pointer;flex:none;line-height:1;}',
    '.ruva-newtab:hover{background:#292b2e;color:#e8eaed;}',
    '#ruva-navbar{display:flex;align-items:center;height:'+NAV_H+'px;padding:0 8px;gap:6px;background:#35363a;}',
    '#ruva-navbar .rbtn{background:transparent;border:none;color:#bdc1c6;width:30px;height:30px;border-radius:50%;cursor:pointer;font-size:15px;line-height:1;display:flex;align-items:center;justify-content:center;flex:none;}',
    '#ruva-navbar .rbtn:hover{background:rgba(255,255,255,0.1);color:#e8eaed;}',
    '#ruva-urlbar{flex:1;background:#202124;border:1px solid transparent;color:#e8eaed;padding:0 14px;border-radius:15px;font-size:13px;height:30px;outline:none;min-width:0;}',
    '#ruva-urlbar:focus{border-color:#8ab4f8;background:#28292c;}'
  ].join('\n');
  wrap.appendChild(style);

  var tabbar=document.createElement('div');
  tabbar.id='ruva-tabbar';
  wrap.appendChild(tabbar);

  var navbar=document.createElement('div');
  navbar.id='ruva-navbar';
  wrap.appendChild(navbar);

  function mkBtn(label,title,cmd){
    var b=document.createElement('button');
    b.className='rbtn';b.textContent=label;b.title=title;
    b.addEventListener('click',function(){post(cmd);});
    return b;
  }
  navbar.appendChild(mkBtn('\u2190','Назад',{cmd:'back'}));
  navbar.appendChild(mkBtn('\u2192','Вперёд',{cmd:'forward'}));
  navbar.appendChild(mkBtn('\u27F3','Обновить',{cmd:'reload'}));
  navbar.appendChild(mkBtn('\u2302','Домой',{cmd:'navigate',url:''}));

  var urlbar=document.createElement('input');
  urlbar.id='ruva-urlbar';
  urlbar.placeholder='Поиск или адрес...';
  if(location.protocol==='http:'||location.protocol==='https:') urlbar.value=location.href;
  urlbar.addEventListener('keydown',function(e){
    if(e.key==='Enter'&&!(e.isComposing||e.keyCode===229)){
      post({cmd:'navigate',url:urlbar.value});
      urlbar.blur();
    }
  });
  urlbar.addEventListener('focus',function(){urlbar.select();});
  navbar.appendChild(urlbar);
  navbar.appendChild(mkBtn('\u2630','Настройки',{cmd:'open_settings'}));

  function applyOffset(){
    var h=NAV_H+(tabsVisible?TABS_H:0);
    tabbar.style.display=tabsVisible?'flex':'none';
    document.body.style.marginTop=h+'px';
  }

  function renderTabs(data){
    tabsVisible=data.show!==false;
    tabbar.innerHTML='';
    (data.tabs||[]).forEach(function(t){
      var el=document.createElement('div');
      el.className='ruva-tab'+(t.active?' active':'');
      el.title=t.title||'Новая вкладка';
      var lbl=document.createElement('span');
      lbl.className='ruva-tab-title';
      lbl.textContent=t.title||'Новая вкладка';
      el.appendChild(lbl);
      var x=document.createElement('button');
      x.className='ruva-tab-close';x.textContent='\u2715';x.title='Закрыть';
      x.addEventListener('click',function(e){e.stopPropagation();post({cmd:'close_tab',id:t.id});});
      el.appendChild(x);
      el.addEventListener('click',function(){if(!t.active)post({cmd:'switch_tab',id:t.id});});
      tabbar.appendChild(el);
    });
    var plus=document.createElement('button');
    plus.className='ruva-newtab';plus.textContent='+';plus.title='Новая вкладка (Ctrl+T)';
    plus.addEventListener('click',function(){post({cmd:'new_tab'});});
    tabbar.appendChild(plus);
    applyOffset();
  }

  window.__ruvaSetTabs=renderTabs;

  document.body.prepend(wrap);
  applyOffset();

  // Report the page title so the tab strip stays in sync.
  function reportTitle(){
    if(document.title)post({cmd:'set_title',title:document.title});
  }
  if(document.title)reportTitle();
  window.addEventListener('load',reportTitle);
  var tEl=document.querySelector('title');
  if(tEl&&window.MutationObserver){
    new MutationObserver(reportTitle).observe(tEl,{childList:true,characterData:true,subtree:true});
  }

  // Keyboard shortcuts.
  document.addEventListener('keydown',function(e){
    if(!e.ctrlKey||e.altKey||e.shiftKey)return;
    var k=e.key.toLowerCase();
    if(k==='t'){e.preventDefault();post({cmd:'new_tab'});}
    else if(k==='w'){e.preventDefault();post({cmd:'close_tab'});}
    else if(k==='l'){e.preventDefault();urlbar.focus();}
    else if(k==='r'){e.preventDefault();post({cmd:'reload'});}
  },true);

  // Ask the backend for the current tab list.
  post({cmd:'get_tabs'});
})();
