// Content-page helper: reports the page title to the tab strip and wires
// keyboard shortcuts. The browser chrome itself lives in a separate
// toolbar webview, so nothing is injected into the page DOM.
(function(){
  if(window.__ruvaHelper)return;
  window.__ruvaHelper=true;

  function post(obj){
    var m=JSON.stringify(obj);
    try{
      if(window.ipc&&window.ipc.postMessage){window.ipc.postMessage(m);return;}
      if(window.chrome&&window.chrome.webview&&window.chrome.webview.postMessage){window.chrome.webview.postMessage(m);return;}
      if(window.webkit&&window.webkit.messageHandlers&&window.webkit.messageHandlers.ipc){window.webkit.messageHandlers.ipc.postMessage(m);return;}
    }catch(e){}
  }

  var lastTitle='';
  function reportTitle(){
    if(document.title&&document.title!==lastTitle){
      lastTitle=document.title;
      post({cmd:'set_title',title:document.title});
    }
  }

  function init(){
    reportTitle();
    var tEl=document.querySelector('title');
    if(tEl&&window.MutationObserver){
      new MutationObserver(reportTitle).observe(tEl,{childList:true,characterData:true,subtree:true});
    }
  }
  if(document.readyState==='loading'){document.addEventListener('DOMContentLoaded',init);}else{init();}
  window.addEventListener('load',reportTitle);

  document.addEventListener('keydown',function(e){
    if(!e.ctrlKey||e.altKey||e.shiftKey)return;
    var k=(e.key||'').toLowerCase();
    if(k==='t'){e.preventDefault();post({cmd:'new_tab'});}
    else if(k==='w'){e.preventDefault();post({cmd:'close_tab'});}
    else if(k==='l'){e.preventDefault();post({cmd:'focus_url'});}
    else if(k==='r'){e.preventDefault();post({cmd:'reload'});}
  },true);
})();
