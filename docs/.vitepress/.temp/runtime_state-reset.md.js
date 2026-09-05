import { ssrRenderAttrs, ssrRenderStyle } from "vue/server-renderer";
import { useSSRContext } from "vue";
import { _ as _export_sfc } from "./plugin-vue_export-helper.1tPrXgE0.js";
const __pageData = JSON.parse('{"title":"Request Lifecycle & State Reset","description":"","frontmatter":{},"headers":[],"relativePath":"runtime/state-reset.md","filePath":"runtime/state-reset.md","lastUpdated":1788590437000}');
const _sfc_main = { name: "runtime/state-reset.md" };
function _sfc_ssrRender(_ctx, _push, _parent, _attrs, $props, $setup, $data, $options) {
  _push(`<div${ssrRenderAttrs(_attrs)}><h1 id="request-lifecycle-state-reset" tabindex="-1">Request Lifecycle &amp; State Reset <a class="header-anchor" href="#request-lifecycle-state-reset" aria-label="Permalink to &quot;Request Lifecycle &amp; State Reset&quot;">​</a></h1><p>In persistent application servers, the most critical architectural requirement is <strong>preventing memory and state leakage across consecutive requests</strong>.</p><hr><h2 id="the-request-lifecycle-pipeline" tabindex="-1">The Request Lifecycle Pipeline <a class="header-anchor" href="#the-request-lifecycle-pipeline" aria-label="Permalink to &quot;The Request Lifecycle Pipeline&quot;">​</a></h2><p>For every HTTP request handled by a persistent worker, RestPHP orchestrates the following strict lifecycle:</p><div class="language-mermaid vp-adaptive-theme"><button title="Copy Code" class="copy"></button><span class="lang">mermaid</span><pre class="shiki shiki-themes github-light github-dark vp-code" tabindex="0"><code><span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">sequenceDiagram</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    participant S as Async HTTP Server (Axum)</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    participant W as Persistent Worker Thread</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    participant C as Custom SAPI Bridge (C)</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    participant Z as Zend Engine Core</span></span>
<span class="line"></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    S-&gt;&gt;W: Dispatch WorkerJob</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    W-&gt;&gt;C: restphp_set_request_info()</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    W-&gt;&gt;Z: php_request_startup_safe()</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    Note over Z: Activate request memory &amp; symbol table</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    W-&gt;&gt;Z: Execute script / handler callback</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    Note over Z: Echoes streamed to ub_write buffer</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    Note over Z: Headers captured by send_headers</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    W-&gt;&gt;Z: php_request_shutdown_safe()</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    Note over Z: Destroy request symbol table &amp; free request heap</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    W-&gt;&gt;S: Deliver PhpResponse via oneshot channel</span></span></code></pre></div><hr><h2 id="state-isolation-guarantees" tabindex="-1">State Isolation Guarantees <a class="header-anchor" href="#state-isolation-guarantees" aria-label="Permalink to &quot;State Isolation Guarantees&quot;">​</a></h2><p>RestPHP guarantees strict request isolation:</p><ol><li><strong>Superglobal Teardown</strong>: Between requests, <code>$_GET</code>, <code>$_POST</code>, <code>$_SERVER</code>, and <code>$_COOKIE</code> are destroyed and recreated with clean request scopes.</li><li><strong>Global Variable Deallocation</strong>: Any global variables defined in userland scripts (e.g. <code>$GLOBALS[&#39;foo&#39;]</code>) are discarded when <code>php_request_shutdown()</code> runs.</li><li><strong>Bailout Protection</strong>: If a userland script calls <code>exit()</code>, <code>die()</code>, or triggers a PHP fatal error, RestPHP intercepts the Zend Engine longjmp bailout via <code>zend_first_try</code> and <code>zend_catch</code>. The worker thread recovers gracefully and serves subsequent requests without crashing the server process.</li></ol><hr><h2 id="verified-by-60-60-e2e-test-suite" tabindex="-1">Verified by 60/60 E2E Test Suite <a class="header-anchor" href="#verified-by-60-60-e2e-test-suite" aria-label="Permalink to &quot;Verified by 60/60 E2E Test Suite&quot;">​</a></h2><p>State reset and memory isolation are verified by RestPHP&#39;s automated test suite:</p><ul><li><strong>Tier 1 Lifecycle Tests</strong>: Consecutive requests verify zero cross-request query leakage.</li><li><strong>Tier 2 Boundary Tests</strong>: Rapid alternating HTTP methods (GET, POST, PUT, DELETE) transition cleanly.</li><li><strong>Tier 4 Stress Tests</strong>: 100 concurrent requests across 10 threads verify zero memory corruption or symbol leakage.</li></ul></div>`);
}
const _sfc_setup = _sfc_main.setup;
_sfc_main.setup = (props, ctx) => {
  const ssrContext = useSSRContext();
  (ssrContext.modules || (ssrContext.modules = /* @__PURE__ */ new Set())).add("runtime/state-reset.md");
  return _sfc_setup ? _sfc_setup(props, ctx) : void 0;
};
const stateReset = /* @__PURE__ */ _export_sfc(_sfc_main, [["ssrRender", _sfc_ssrRender]]);
export {
  __pageData,
  stateReset as default
};
