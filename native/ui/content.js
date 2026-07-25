// Injected into every content page (websites, NTP, settings, history).
// Reports HTML fullscreen changes and global keyboard shortcuts to the shell.
(function () {
  function send(o) {
    try { window.ipc.postMessage(JSON.stringify(o)); } catch (e) {}
  }

  document.addEventListener('fullscreenchange', function () {
    send({ cmd: 'fullscreen', on: !!document.fullscreenElement });
  });

  document.addEventListener('keydown', function (e) {
    var k = (e.key || '').toLowerCase();
    if (e.ctrlKey && !e.shiftKey && !e.altKey) {
      if (k === 't') { e.preventDefault(); send({ cmd: 'key', key: 'new_tab' }); }
      else if (k === 'w') { e.preventDefault(); send({ cmd: 'key', key: 'close_tab' }); }
      else if (k === 'l') { e.preventDefault(); send({ cmd: 'key', key: 'focus_url' }); }
      else if (k === 'h') { e.preventDefault(); send({ cmd: 'key', key: 'history' }); }
      else if (k === 'tab') { e.preventDefault(); send({ cmd: 'key', key: 'next_tab' }); }
    }
    if (e.key === 'F11') { e.preventDefault(); send({ cmd: 'key', key: 'f11' }); }
  }, true);
})();
