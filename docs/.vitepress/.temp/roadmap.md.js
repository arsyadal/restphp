import { ssrRenderAttrs } from "vue/server-renderer";
import { useSSRContext } from "vue";
import { _ as _export_sfc } from "./plugin-vue_export-helper.1tPrXgE0.js";
const __pageData = JSON.parse('{"title":"Project Roadmap & Milestones","description":"","frontmatter":{},"headers":[],"relativePath":"roadmap.md","filePath":"roadmap.md","lastUpdated":1788589921000}');
const _sfc_main = { name: "roadmap.md" };
function _sfc_ssrRender(_ctx, _push, _parent, _attrs, $props, $setup, $data, $options) {
  _push(`<div${ssrRenderAttrs(_attrs)}><h1 id="project-roadmap-milestones" tabindex="-1">Project Roadmap &amp; Milestones <a class="header-anchor" href="#project-roadmap-milestones" aria-label="Permalink to &quot;Project Roadmap &amp; Milestones&quot;">​</a></h1><p>The development of <strong>RestPHP</strong> follows a strict multi-phase architecture blueprint designed for production resilience and world-record performance.</p><hr><h2 id="progress-overview" tabindex="-1">Progress Overview <a class="header-anchor" href="#progress-overview" aria-label="Permalink to &quot;Progress Overview&quot;">​</a></h2><ul><li><p>[x] <strong>Milestone 1: Zend Engine C-FFI Core Embedding</strong></p><ul><li>Toolchain setup (<code>libphp-embed</code>, <code>clang</code>, <code>rustc</code>).</li><li>Raw C bindings for <code>php_embed_init</code>, <code>zend_eval_string</code>, and <code>php_embed_shutdown</code>.</li><li>In-memory execution verified.</li></ul></li><li><p>[x] <strong>Milestone 2: Custom SAPI Implementation</strong></p><ul><li>Dedicated <code>sapi_module_struct</code> implementation.</li><li>Intercepted output buffering (<code>ub_write</code>) to stream directly to Rust memory buffers.</li><li>Header capturing (<code>send_headers</code>) for HTTP status codes and custom headers.</li><li>Superglobal injection (<code>$_SERVER</code>, <code>$_GET</code>, <code>$_POST</code>, <code>$_COOKIE</code>, <code>php://input</code>).</li></ul></li><li><p>[x] <strong>Milestone 3: Async Tokio HTTP Server &amp; REST Engine</strong></p><ul><li>Tokio + Axum async HTTP listener.</li><li>Lock-free crossbeam worker dispatcher.</li><li>CLI commands (<code>serve</code>, <code>eval</code>).</li><li>Verified with live HTTP traffic.</li></ul></li><li><p>[x] <strong>Milestone 4: Persistent Worker Loop &amp; Laravel Octane Adapter</strong></p><ul><li>Per-request lifecycle (<code>php_request_startup</code> -&gt; handler -&gt; <code>php_request_shutdown</code>).</li><li>State reset verified across 60/60 E2E tests (100% pass rate).</li><li>Released <code>restphp/octane</code> Composer package for 1st-class Laravel integration.</li></ul></li><li><p>[ ] <strong>Milestone 5: Benchmarking Suite &amp; TechEmpower</strong></p><ul><li>Automated comparative benchmarks vs FrankenPHP and Swoole.</li><li>Hot code reload via <code>notify</code> crate.</li><li>Official TechEmpower Framework Benchmarks submission.</li></ul></li></ul></div>`);
}
const _sfc_setup = _sfc_main.setup;
_sfc_main.setup = (props, ctx) => {
  const ssrContext = useSSRContext();
  (ssrContext.modules || (ssrContext.modules = /* @__PURE__ */ new Set())).add("roadmap.md");
  return _sfc_setup ? _sfc_setup(props, ctx) : void 0;
};
const roadmap = /* @__PURE__ */ _export_sfc(_sfc_main, [["ssrRender", _sfc_ssrRender]]);
export {
  __pageData,
  roadmap as default
};
