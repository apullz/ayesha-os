(function () {
  'use strict';

  var STRUCT = {{STRUCT_JSON}};
  var WELCOME = 'Hello Fox! \u2728 What magical adventure should we dream up today?';

  var STORE = [];
  var BUSY = false;
  var BOOTED = false;

  var API_NAME = 'respond';
  var API_READY = fetch('/config')
    .then(function (r) { return r.json(); })
    .then(function (cfg) {
      var deps = cfg.dependencies || [];
      for (var i = 0; i < deps.length; i++) {
        if (deps[i].api_name === 'respond') { API_NAME = 'respond'; return; }
      }
      for (var j = 0; j < deps.length; j++) {
        var d = deps[j];
        var n = d.api_name || '';
        if (
          n && n[0] !== '_' && n.indexOf('lambda') !== 0 && n.indexOf('unnamed') !== 0 &&
          (d.inputs || []).length === 2 && (d.outputs || []).length === 2
        ) {
          API_NAME = n;
          return;
        }
      }
    })
    .catch(function () { /* keep default 'respond' */ });

  function $(id) {
    return document.getElementById(id);
  }

  function boot() {
    if (BOOTED) return;
    if (document.readyState === 'complete') {
      init();
    } else {
      window.addEventListener('load', init);
    }
  }

  function init() {
    if (BOOTED) return;
    if (document.getElementById('ayesha-overlay')) {
      BOOTED = true;
      return;
    }
    var app = document.getElementById('app');
    if (!app) {
      setTimeout(init, 200);
      return;
    }
    BOOTED = true;
    app.style.display = 'none';
    var loading = document.getElementById('loading');
    if (loading) loading.style.display = 'none';

    var root = document.createElement('div');
    root.id = 'ayesha-overlay';
    root.innerHTML = STRUCT;
    document.body.appendChild(root);
    document.body.style.margin = '0';
    document.title = 'Ayesha Bot \u2014 Magical Chat';

    buildSparkles(root);
    wire(root);
    STORE = [{ role: 'bot', content: WELCOME }];
    renderBubble('bot', WELCOME, false);
    scrollBottom();
  }

  function buildSparkles(root) {
    var holder = root.querySelector('.a-sparkles');
    if (!holder) return;
    var seed = 7;
    function rand() {
      seed = (seed * 9301 + 49297) % 233280;
      return seed / 233280;
    }
    var NS = 'http://www.w3.org/2000/svg';
    var frag = document.createDocumentFragment();
    var i, svg, path, style;
    for (i = 0; i < 16; i++) {
      svg = document.createElementNS(NS, 'svg');
      svg.setAttribute('viewBox', '0 0 24 24');
      svg.setAttribute('class', 'a-star');
      path = document.createElementNS(NS, 'path');
      path.setAttribute('d', 'M12 0c.6 4.9 2.1 6.4 7 7-4.9.6-6.4 2.1-7 7-.6-4.9-2.1-6.4-7-7 4.9-.6 6.4-2.1 7-7z');
      svg.appendChild(path);
      style = svg.style;
      style.left = Math.round(rand() * 100) + '%';
      style.top = Math.round(rand() * 100) + '%';
      style.width = (8 + Math.round(rand() * 16)) + 'px';
      style.height = (8 + Math.round(rand() * 16)) + 'px';
      style.animationDelay = (rand() * 2.4).toFixed(2) + 's';
      style.animationDuration = (2 + rand() * 2).toFixed(2) + 's';
      frag.appendChild(svg);
    }
    holder.appendChild(frag);
  }

  function wire(root) {
    var form = $('a-form');
    var input = $('a-input');
    var starBtn = root.querySelector('.a-star-btn');

    form.addEventListener('submit', function (e) {
      e.preventDefault();
      submit(input);
    });
    input.addEventListener('keydown', function (e) {
      if (e.key === 'Enter' && !e.isComposing && e.keyCode !== 229) {
        e.preventDefault();
        submit(input);
      }
    });
    starBtn.addEventListener('click', function () {
      starBtn.classList.toggle('is-fav');
    });

    var tabs = root.querySelectorAll('.a-nav-item');
    tabs.forEach(function (tab) {
      tab.addEventListener('click', function () {
        tabs.forEach(function (t) {
          t.classList.remove('is-active');
          t.removeAttribute('aria-current');
        });
        tab.classList.add('is-active');
        tab.setAttribute('aria-current', 'page');
      });
    });
  }

  function submit(input) {
    var text = input.value.trim();
    if (!text || BUSY) return;
    input.value = '';
    STORE.push({ role: 'user', content: text });
    renderBubble('user', text, false);
    var typingEl = renderBubble('bot', '', true);
    BUSY = true;
    $('a-send').classList.add('a-send-busy');
    scrollBottom();

    stream(text, STORE.slice(0, -1), {
      onChunk: function (t) {
        if (typingEl.classList.contains('a-typing')) {
          typingEl.classList.remove('a-typing');
        }
        typingEl.textContent = t;
        scrollBottom();
      },
      onDone: function () {
        if (typingEl.classList.contains('a-typing')) {
          typingEl.classList.remove('a-typing');
          typingEl.textContent = '';
        }
        STORE.push({ role: 'assistant', content: typingEl.textContent || '' });
        BUSY = false;
        $('a-send').classList.remove('a-send-busy');
        scrollBottom();
      },
      onError: function (err) {
        if (typingEl.classList.contains('a-typing')) {
          typingEl.classList.remove('a-typing');
        }
        typingEl.textContent = '[connection hiccup: ' + err + ']';
        BUSY = false;
        $('a-send').classList.remove('a-send-busy');
      }
    });
  }

  function renderBubble(role, text, isTyping) {
    var scroll = $('a-scroll');
    var row = document.createElement('div');
    row.className = 'a-row ' + (role === 'user' ? 'a-row-user' : 'a-row-bot');

    var bubble = document.createElement('div');
    bubble.className = 'a-bubble ' + (role === 'user' ? 'a-bubble-user' : 'a-bubble-bot');
    bubble.setAttribute('role', role === 'user' ? 'user' : 'assistant');

    var tail = document.createElement('span');
    tail.className = 'a-tail';
    tail.setAttribute('aria-hidden', 'true');
    bubble.appendChild(tail);

    var textEl = document.createElement('div');
    textEl.className = 'a-bubble-text';
    if (isTyping) {
      textEl.classList.add('a-typing');
      textEl.innerHTML = '<span></span><span></span><span></span>';
    } else {
      textEl.textContent = text;
    }
    bubble.appendChild(textEl);
    row.appendChild(bubble);
    scroll.appendChild(row);
    return textEl;
  }

  function scrollBottom() {
    var scroll = $('a-scroll');
    if (scroll) scroll.scrollTop = scroll.scrollHeight;
  }

  function stream(message, history, handlers) {
    API_READY.then(function () {
      var url = '/gradio_api/call/' + API_NAME;
      return fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ data: [message, history] })
      })
        .then(function (r) {
          if (!r.ok) throw new Error('HTTP ' + r.status);
          return r.json();
        })
        .then(function (init) {
          return fetch(url + '/' + init.event_id, {
            headers: { 'Accept': 'text/event-stream' }
          });
        })
        .then(function (res) {
          if (!res.ok) throw new Error('HTTP ' + res.status);
          return consume(res, handlers);
        });
    }).catch(function (err) {
      handlers.onError(err && err.message ? err.message : String(err));
    });
  }

  function consume(res, handlers) {
    var reader = res.body.getReader();
    var decoder = new TextDecoder();
    var buf = '';
    var eventName = '';
    var dataLines = [];

    function dispatch() {
      var payload = dataLines.join('\n');
      dataLines = [];
      var name = eventName;
      eventName = '';
      if (name === 'generating') {
        var t = payload;
        try {
          var arr = JSON.parse(payload);
          if (Array.isArray(arr) && typeof arr[0] === 'string') t = arr[0];
        } catch (e) { /* keep raw */ }
        if (t) handlers.onChunk(t);
      } else if (name === 'complete') {
        handlers.onDone();
      }
    }

    function pump() {
      return reader.read().then(function (r) {
        if (r.done) {
          dispatch();
          handlers.onDone();
          return;
        }
        buf += decoder.decode(r.value, { stream: true });
        var idx;
        while ((idx = buf.indexOf('\n')) >= 0) {
          var line = buf.slice(0, idx).replace(/\r$/, '');
          buf = buf.slice(idx + 1);
          if (line === '') {
            dispatch();
          } else if (line.indexOf('event:') === 0) {
            eventName = line.slice(6).trim();
          } else if (line.indexOf('data:') === 0) {
            dataLines.push(line.slice(5).trim());
          }
        }
        return pump();
      });
    }
    return pump();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }
})();
