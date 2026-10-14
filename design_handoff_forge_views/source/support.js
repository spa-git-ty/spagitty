// SPDX-License-Identifier: GPL-3.0-or-later
//
// A small stand-alone viewer for the .dc.html artboards in this folder, so each
// one opens straight from disk in a browser. It is not part of Spagitty and
// nothing in the app should import it.
//
// What it does: reads the <x-dc> template and the <script type="text/x-dc">
// logic, then renders the template with the values from renderVals():
// {{ path }} holes, <sc-for list as>, <sc-if value>, onClick/onChange
// handlers, and setState() re-renders. <helmet> moves into <head>.
//
// Props: each data-props default, overridable from the URL —
// e.g. Recipes.dc.html?theme=light.

(function () {
  'use strict';

  var hide = document.createElement('style');
  hide.textContent = 'x-dc{display:block;visibility:hidden}';
  document.head.appendChild(hide);

  var HOLE = /\{\{\s*([^}]+?)\s*\}\}/g;
  var WHOLE = /^\s*\{\{\s*([^}]+?)\s*\}\}\s*$/;

  function literal(path) {
    if (path === 'true') return { ok: true, v: true };
    if (path === 'false') return { ok: true, v: false };
    if (path === 'null') return { ok: true, v: null };
    if (/^-?\d+(\.\d+)?$/.test(path)) return { ok: true, v: Number(path) };
    var q = /^(['"])(.*)\1$/.exec(path);
    if (q) return { ok: true, v: q[2] };
    return { ok: false };
  }

  function resolve(path, scopes) {
    var lit = literal(path);
    if (lit.ok) return lit.v;
    var parts = path.split('.');
    var head = parts[0];
    for (var i = scopes.length - 1; i >= 0; i--) {
      var s = scopes[i];
      if (s != null && Object.prototype.hasOwnProperty.call(s, head)) {
        var v = s[head];
        for (var j = 1; j < parts.length; j++) {
          if (v == null) return undefined;
          v = v[parts[j]];
        }
        return v;
      }
    }
    return undefined;
  }

  function text(v) {
    return v == null || v === false ? '' : String(v);
  }

  function interpolate(str, scopes) {
    return str.replace(HOLE, function (_, p) {
      return text(resolve(p, scopes));
    });
  }

  function children(node, scopes, out) {
    for (var c = node.firstChild; c; c = c.nextSibling) build(c, scopes, out);
  }

  function build(node, scopes, out) {
    if (node.nodeType === 3) {
      out.appendChild(document.createTextNode(interpolate(node.nodeValue, scopes)));
      return;
    }
    if (node.nodeType !== 1) return;
    var tag = node.localName.toLowerCase();

    if (tag === 'helmet') return;

    if (tag === 'sc-for') {
      var list = resolve((WHOLE.exec(node.getAttribute('list') || '') || [])[1] || '', scopes) || [];
      var as = node.getAttribute('as') || 'item';
      for (var i = 0; i < list.length; i++) {
        var scope = { $index: i };
        scope[as] = list[i];
        children(node, scopes.concat([scope]), out);
      }
      return;
    }

    if (tag === 'sc-if') {
      var m = WHOLE.exec(node.getAttribute('value') || '');
      if (m && resolve(m[1], scopes)) children(node, scopes, out);
      return;
    }

    var el = document.createElementNS(node.namespaceURI, node.localName);
    var value;
    for (var a = 0; a < node.attributes.length; a++) {
      var attr = node.attributes[a];
      var name = attr.name;
      if (name.indexOf('hint-') === 0) continue;
      var whole = WHOLE.exec(attr.value);
      if (/^on[a-z]+$/i.test(name)) {
        var fn = whole ? resolve(whole[1], scopes) : null;
        if (typeof fn === 'function') {
          var type = name.slice(2).toLowerCase();
          // React's onChange fires on every keystroke.
          el.addEventListener(type === 'change' ? 'input' : type, fn);
        }
        continue;
      }
      var v = whole ? resolve(whole[1], scopes) : interpolate(attr.value, scopes);
      if (name === 'value') {
        value = v;
        continue;
      }
      if (v === undefined || v === null || v === false) continue;
      el.setAttribute(name, v === true ? '' : String(v));
    }
    children(node, scopes, el);
    if (value !== undefined) {
      el.value = text(value);
      if (tag === 'textarea') el.textContent = text(value);
    }
    out.appendChild(el);
  }

  // Keep the caret where it was across a re-render.
  function pathOf(el, root) {
    var path = [];
    while (el && el !== root) {
      var p = el.parentNode;
      if (!p) return null;
      path.unshift(Array.prototype.indexOf.call(p.childNodes, el));
      el = p;
    }
    return el === root ? path : null;
  }

  function atPath(root, path) {
    var el = root;
    for (var i = 0; i < path.length && el; i++) el = el.childNodes[path[i]];
    return el;
  }

  function start() {
    var host = document.querySelector('x-dc');
    var logic = document.querySelector('script[data-dc-script]');
    if (!host || !logic) return;

    var helmet = host.querySelector('helmet');
    if (helmet) {
      while (helmet.firstChild) document.head.appendChild(helmet.firstChild);
    }

    var template = document.createElement('div');
    while (host.firstChild) template.appendChild(host.firstChild);

    var props = {};
    var declared = {};
    try {
      declared = JSON.parse(logic.getAttribute('data-props') || '{}');
    } catch (e) {
      console.error('data-props is not JSON', e);
    }
    Object.keys(declared).forEach(function (k) {
      if (k.charAt(0) !== '$' && declared[k] && 'default' in declared[k]) props[k] = declared[k].default;
    });
    new URLSearchParams(location.search).forEach(function (v, k) {
      props[k] = v === 'true' ? true : v === 'false' ? false : v;
    });

    var queued = false;
    var component;

    function DCLogic(p) {
      this.props = p;
      this.state = {};
    }
    DCLogic.prototype.setState = function (patch) {
      var next = typeof patch === 'function' ? patch(this.state, this.props) : patch;
      this.state = Object.assign({}, this.state, next);
      schedule();
    };
    DCLogic.prototype.forceUpdate = function () {
      schedule();
    };

    function schedule() {
      if (queued) return;
      queued = true;
      Promise.resolve().then(function () {
        queued = false;
        render();
      });
    }

    function render() {
      var active = document.activeElement;
      var focusPath = active && host.contains(active) ? pathOf(active, host) : null;
      var sel = null;
      if (focusPath && 'selectionStart' in active) {
        try {
          sel = [active.selectionStart, active.selectionEnd];
        } catch (e) {
          sel = null;
        }
      }

      var vals;
      try {
        vals = component.renderVals() || {};
      } catch (e) {
        console.error('renderVals failed', e);
        return;
      }
      var frag = document.createDocumentFragment();
      children(template, [vals], frag);
      host.replaceChildren(frag);

      if (focusPath) {
        var again = atPath(host, focusPath);
        if (again && again.focus) {
          again.focus();
          if (sel && again.setSelectionRange) {
            try {
              again.setSelectionRange(sel[0], sel[1]);
            } catch (e) {
              /* not a text field */
            }
          }
        }
      }
    }

    var Component;
    try {
      Component = new Function('DCLogic', logic.textContent + '\n;return Component;')(DCLogic);
    } catch (e) {
      console.error('The artboard script did not load', e);
      return;
    }
    component = new Component(props);
    if (!component.props) component.props = props;
    if (!component.state) component.state = {};
    render();
    hide.remove();
    if (typeof component.componentDidMount === 'function') component.componentDidMount();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
})();
