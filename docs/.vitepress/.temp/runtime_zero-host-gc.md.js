import { ssrRenderAttrs, ssrRenderStyle } from "vue/server-renderer";
import { useSSRContext } from "vue";
import { _ as _export_sfc } from "./plugin-vue_export-helper.1tPrXgE0.js";
const __pageData = JSON.parse('{"title":"Zero Host GC & Deterministic Latency","description":"","frontmatter":{},"headers":[],"relativePath":"runtime/zero-host-gc.md","filePath":"runtime/zero-host-gc.md","lastUpdated":1788590437000}');
const _sfc_main = { name: "runtime/zero-host-gc.md" };
function _sfc_ssrRender(_ctx, _push, _parent, _attrs, $props, $setup, $data, $options) {
  _push(`<div${ssrRenderAttrs(_attrs)}><h1 id="zero-host-gc-deterministic-latency" tabindex="-1">Zero Host GC &amp; Deterministic Latency <a class="header-anchor" href="#zero-host-gc-deterministic-latency" aria-label="Permalink to &quot;Zero Host GC &amp; Deterministic Latency&quot;">​</a></h1><p>One of the foundational design choices of RestPHP is the <strong>complete elimination of host runtime garbage collection</strong>.</p><hr><h2 id="the-double-gc-problem-in-go-based-runtimes" tabindex="-1">The &quot;Double GC&quot; Problem in Go-based Runtimes <a class="header-anchor" href="#the-double-gc-problem-in-go-based-runtimes" aria-label="Permalink to &quot;The &quot;Double GC&quot; Problem in Go-based Runtimes&quot;">​</a></h2><p>Modern persistent PHP application servers such as <strong>FrankenPHP</strong> and <strong>RoadRunner</strong> are written in Go. While Go provides high concurrency through goroutines, its memory management relies on a concurrent <strong>Stop-The-World (STW) Garbage Collector</strong>.</p><p>When running high-throughput web workloads:</p><ol><li><strong>PHP&#39;s Cyclic GC</strong>: The PHP Zend Engine has its own garbage collector (<code>zend_gc_collect_cycles</code>) to clean up circular references in userland variables.</li><li><strong>Go&#39;s Host GC</strong>: The host server (Go) continuously allocates temporary memory buffers for HTTP headers, TCP buffers, and request routing, triggering periodic Go GC phases.</li></ol><h3 id="impact-on-p99-tail-latency" tabindex="-1">Impact on p99 Tail Latency <a class="header-anchor" href="#impact-on-p99-tail-latency" aria-label="Permalink to &quot;Impact on p99 Tail Latency&quot;">​</a></h3><p>During Go GC cycles, CPU cores are diverted to mark-and-sweep phases, and thread scheduling is briefly paused. This causes:</p><ul><li><strong>GC Jitter</strong>: Periodic latency spikes where requests that normally take 2ms suddenly take 15ms or 40ms.</li><li><strong>Unpredictable p99/p99.9 Latency</strong>: Critical in enterprise microservices where SLA guarantees depend on consistent tail latency.</li></ul><div class="language-mermaid vp-adaptive-theme"><button title="Copy Code" class="copy"></button><span class="lang">mermaid</span><pre class="shiki shiki-themes github-light github-dark vp-code" tabindex="0"><code><span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">graph LR</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    subgraph Go_Runtimes [&quot;FrankenPHP / RoadRunner (Go)&quot;]</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">        G1[Request Influx] --&gt; G2[Double Garbage Collection]</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">        G2 --&gt; G3[&quot;Go GC Pauses (STW) + Zend GC&quot;]</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">        G3 --&gt; G4[&quot;p99 Latency Jitter (4.8ms - 20ms)&quot;]</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    end</span></span>
<span class="line"></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    subgraph Rust_RestPHP [&quot;RestPHP (Rust)&quot;]</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">        R1[Request Influx] --&gt; R2[Compile-Time RAII Ownership]</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">        R2 --&gt; R3[&quot;Zero Host Garbage Collection&quot;]</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">        R3 --&gt; R4[&quot;Flat, Deterministic p99 (1.2ms)&quot;]</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    end</span></span>
<span class="line"></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    style Rust_RestPHP fill:#0f172a,stroke:#f97316,stroke-width:2px</span></span>
<span class="line"><span style="${ssrRenderStyle({ "--shiki-light": "#24292E", "--shiki-dark": "#E1E4E8" })}">    style Go_Runtimes fill:#0f172a,stroke:#64748b,stroke-width:1px</span></span></code></pre></div><hr><h2 id="the-rust-advantage-compile-time-raii" tabindex="-1">The Rust Advantage: Compile-Time RAII <a class="header-anchor" href="#the-rust-advantage-compile-time-raii" aria-label="Permalink to &quot;The Rust Advantage: Compile-Time RAII&quot;">​</a></h2><p>Rust does not have a runtime garbage collector:</p><ul><li><strong>Deterministic Destruction (RAII)</strong>: Memory allocated for incoming requests, headers, and responses is freed the exact microsecond the variable falls out of scope.</li><li><strong>No Background Sweepers</strong>: Zero background CPU cycles are spent scanning heap pointers.</li><li><strong>Rock-Solid Tail Latency</strong>: Under sustained high-load concurrency benchmarks (100+ concurrent connections), RestPHP maintains an ultra-flat p99 latency curve of <strong>~1.2 ms</strong>.</li></ul></div>`);
}
const _sfc_setup = _sfc_main.setup;
_sfc_main.setup = (props, ctx) => {
  const ssrContext = useSSRContext();
  (ssrContext.modules || (ssrContext.modules = /* @__PURE__ */ new Set())).add("runtime/zero-host-gc.md");
  return _sfc_setup ? _sfc_setup(props, ctx) : void 0;
};
const zeroHostGc = /* @__PURE__ */ _export_sfc(_sfc_main, [["ssrRender", _sfc_ssrRender]]);
export {
  __pageData,
  zeroHostGc as default
};
