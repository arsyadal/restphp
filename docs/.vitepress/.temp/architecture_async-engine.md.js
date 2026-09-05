import { ssrRenderAttrs } from "vue/server-renderer";
import { useSSRContext } from "vue";
import { _ as _export_sfc } from "./plugin-vue_export-helper.1tPrXgE0.js";
const __pageData = JSON.parse('{"title":"Async Tokio & Axum Engine","description":"","frontmatter":{},"headers":[],"relativePath":"architecture/async-engine.md","filePath":"architecture/async-engine.md","lastUpdated":1788590437000}');
const _sfc_main = { name: "architecture/async-engine.md" };
function _sfc_ssrRender(_ctx, _push, _parent, _attrs, $props, $setup, $data, $options) {
  _push(`<div${ssrRenderAttrs(_attrs)}><h1 id="async-tokio-axum-engine" tabindex="-1">Async Tokio &amp; Axum Engine <a class="header-anchor" href="#async-tokio-axum-engine" aria-label="Permalink to &quot;Async Tokio &amp; Axum Engine&quot;">​</a></h1><p>The front-end HTTP server of RestPHP is built using <strong>Tokio</strong> and <strong>Axum</strong>, providing asynchronous I/O with near-zero connection overhead.</p><hr><h2 id="high-concurrency-connection-pooling" tabindex="-1">High-Concurrency Connection Pooling <a class="header-anchor" href="#high-concurrency-connection-pooling" aria-label="Permalink to &quot;High-Concurrency Connection Pooling&quot;">​</a></h2><p>In <code>src/server.rs</code>:</p><ul><li>Axum accepts incoming HTTP/1.1 and HTTP/2 connections asynchronously.</li><li>Non-blocking I/O allows a single process to hold open tens of thousands of idle client sockets without thread exhaustion.</li><li>Incoming HTTP requests are converted into <code>WorkerJob</code> envelopes containing method, URI, query parameters, headers, cookies, and body bytes.</li></ul><hr><h2 id="lock-free-request-dispatch" tabindex="-1">Lock-Free Request Dispatch <a class="header-anchor" href="#lock-free-request-dispatch" aria-label="Permalink to &quot;Lock-Free Request Dispatch&quot;">​</a></h2><p>Requests are queued to the worker thread pool using <code>crossbeam-channel</code>:</p><ul><li>Workers poll the bounded channel without mutual-exclusion lock contention.</li><li>Once a worker completes request execution, it transmits the <code>PhpResponse</code> back to Axum&#39;s async task using a lightweight <code>tokio::sync::oneshot</code> channel.</li><li>Axum streams the response status, headers, and body back to the HTTP client with zero duplicate allocations.</li></ul></div>`);
}
const _sfc_setup = _sfc_main.setup;
_sfc_main.setup = (props, ctx) => {
  const ssrContext = useSSRContext();
  (ssrContext.modules || (ssrContext.modules = /* @__PURE__ */ new Set())).add("architecture/async-engine.md");
  return _sfc_setup ? _sfc_setup(props, ctx) : void 0;
};
const asyncEngine = /* @__PURE__ */ _export_sfc(_sfc_main, [["ssrRender", _sfc_ssrRender]]);
export {
  __pageData,
  asyncEngine as default
};
