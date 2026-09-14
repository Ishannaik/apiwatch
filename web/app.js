const $ = (id) => document.getElementById(id);

const FRONTIER = ["openai", "google", "anthropic", "xai", "mistral"];
const LABELS = {
  frontier: "Frontier",
  openai: "OpenAI",
  google: "Gemini",
  anthropic: "Anthropic",
  xai: "xAI",
  mistral: "Mistral",
  groq: "Groq",
  all: "All",
};
const LAB_SOURCES = {
  openai: ["openai_docs", "openai_sdk"],
  google: ["google_docs", "google_llms", "google_vertex"],
  anthropic: ["anthropic_docs", "anthropic_mirror"],
  xai: ["xai_docs", "xai_sdk"],
  mistral: ["mistral_docs", "portkey_mistral"],
  groq: ["groq_docs"],
};

function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  }[c]));
}

function fmtTime(unix) {
  if (!unix) return "—";
  const d = new Date(Number(unix) * 1000);
  return d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

function pill(ok, status, notModified) {
  if (!ok) return `<span class="pill fail">${esc(status || "err")}</span>`;
  if (notModified) return `<span class="pill ok">304</span>`;
  return `<span class="pill ok">${esc(status)}</span>`;
}

function bare(id) {
  let s = String(id);
  const prefixes = [
    "openai/",
    "google-vertex/",
    "google/",
    "vertex_ai/",
    "gemini/",
    "anthropic/",
    "x-ai/",
    "xai/",
    "mistralai/",
    "mistral/",
  ];
  let again = true;
  while (again) {
    again = false;
    const lower = s.toLowerCase();
    for (const p of prefixes) {
      if (lower.startsWith(p)) {
        s = s.slice(p.length);
        again = true;
        break;
      }
    }
  }
  return s;
}

function labOf(id, src) {
  const b = bare(id).toLowerCase();
  if (/^(gpt-|o[1-9]|chatgpt-|codex-|sora-|omni-|computer-use|dall-e)/.test(b)) return "openai";
  if (/^(gemini|gemma|imagen|veo|lyria|antigravity|deep-research)/.test(b)) return "google";
  if (/^claude-/.test(b)) return "anthropic";
  if (/^grok-/.test(b)) return "xai";
  if (
    /^(mistral|codestral|pixtral|ministral|magistral|voxtral|devstral|open-mistral|open-mixtral|open-codestral|labs-)/.test(b)
  ) {
    return "mistral";
  }
  if (String(id).toLowerCase().startsWith("groq/") || (src && (LAB_SOURCES.groq || []).includes(src))) {
    return "groq";
  }
  return null;
}

function isAlias(id) {
  const s = bare(id).toLowerCase();
  return /[@:]/.test(s) || /-(gt|lte)-\d+k/.test(s) || /-(thinking|think|free)$/.test(s);
}

function isOfficial(lab, srcs) {
  const allow = LAB_SOURCES[lab] || [];
  return srcs.some((src) => allow.includes(src));
}

function familyRank(id) {
  const s = bare(id).toLowerCase();
  if (/^gpt-\d/.test(s) && !s.includes("oss")) return 50;
  if (/^o[1-9]/.test(s)) return 45;
  if (/^chatgpt-/.test(s)) return 40;
  if (/^gemini-\d/.test(s)) return 50;
  if (/^claude-(fable|opus|sonnet|haiku)-/.test(s)) return 50;
  if (/^grok-\d/.test(s)) return 50;
  if (/^codex-/.test(s)) return 30;
  if (/^sora-/.test(s)) return 20;
  return 0;
}

function versionParts(id) {
  const s = bare(id);
  const m = s.match(/(\d+)(?:\.(\d+))?/);
  if (!m) return [0, 0];
  return [parseInt(m[1], 10), m[2] ? parseInt(m[2], 10) : 0];
}

function cmpVersion(a, b) {
  const fa = familyRank(a.id);
  const fb = familyRank(b.id);
  if (fa !== fb) return fb - fa;
  const pa = versionParts(a.id);
  const pb = versionParts(b.id);
  if (pa[0] !== pb[0]) return pb[0] - pa[0];
  if (pa[1] !== pb[1]) return pb[1] - pa[1];
  return a.id.localeCompare(b.id);
}

function matchesFilter(lab, filter) {
  if (filter === "all") return true;
  if (filter === "frontier") return FRONTIER.includes(lab);
  return lab === filter;
}

function itemHtml(x, newest) {
  const isNew = newest.has(x.id.toLowerCase());
  return `<li>${isNew ? `<span class="pill ok">new</span> ` : ""}<span class="meta">${esc(x.srcs.join(" · "))}</span> ${esc(x.id)}</li>`;
}

async function load() {
  const res = await fetch("state.json", { cache: "no-store" });
  if (!res.ok) throw new Error("state.json " + res.status);
  const s = await res.json();
  const t = s.totals || {};
  $("stamp").textContent =
    `${t.models ?? "—"} models · ${t.sources_ok ?? 0}/${t.sources ?? 0} sources · ${fmtTime(s.generated_at)} · ${s.scan_ms ?? "—"}ms`;

  const fails = (s.sources || []).filter((x) => !x.ok);
  if (fails.length) {
    $("attention").hidden = false;
    $("attention").textContent = fails.map((x) => `${x.name}: ${x.error || x.status}`).join(" · ");
  }

  $("sources").innerHTML = (s.sources || []).map((x) => `
    <tr>
      <td class="name">${esc(x.name)}<div class="meta">${esc(x.id)}</div></td>
      <td>${Number(x.count) || 0}</td>
      <td>${(x.added || []).length}</td>
      <td>${(x.removed || []).length}</td>
      <td>${pill(x.ok, x.status, x.not_modified)}</td>
      <td>${esc(x.ms ?? "—")}</td>
    </tr>`).join("");

  const rows = new Map();
  for (const [src, list] of Object.entries(s.ids || {})) {
    for (const raw of list) {
      const id = bare(raw);
      const lab = labOf(raw, src) || labOf(id, src);
      const key = `${lab || "other"}\t${id.toLowerCase()}`;
      let row = rows.get(key);
      if (!row) {
        row = { id, lab, srcs: [] };
        rows.set(key, row);
      }
      if (!row.srcs.includes(src)) row.srcs.push(src);
    }
  }
  const all = [...rows.values()];
  const newest = new Set();
  for (const e of s.events || []) {
    if (e.op === "add") newest.add(bare(e.id).toLowerCase());
  }
  for (const src of s.sources || []) {
    for (const id of src.added || []) newest.add(bare(id).toLowerCase());
  }

  let filter = "frontier";
  const render = () => {
    const needle = ($("q").value || "").trim().toLowerCase();
    $("models-heading").textContent = LABELS[filter] || filter;

    const shownEvents = (s.events || []).filter((e) => {
      const lab = labOf(e.id, e.source);
      if (!matchesFilter(lab, filter)) return false;
      if (!needle) return true;
      return String(e.id).toLowerCase().includes(needle) || String(e.source).includes(needle);
    });
    $("events").innerHTML = shownEvents.length
      ? shownEvents.slice(0, 40).map((e) =>
          `<li class="${e.op === "remove" ? "remove" : "add"}"><span class="meta">${esc(fmtTime(e.ts))} ${esc(e.source)}</span> ${e.op === "add" ? "+" : "−"} ${esc(e.id)}</li>`
        ).join("")
      : `<li class="meta">${s.baseline ? "Baseline scan — next diff will show adds/removes." : "No changes in this view."}</li>`;

    const picked = all.filter((x) => {
      if (!matchesFilter(x.lab, filter)) return false;
      if (filter !== "all") {
        if (isAlias(x.id)) return false;
        if (!isOfficial(x.lab, x.srcs)) return false;
      }
      if (!needle) return true;
      return x.id.toLowerCase().includes(needle) || x.srcs.some((src) => src.includes(needle));
    });
    picked.sort((a, b) => {
      const an = newest.has(a.id.toLowerCase()) ? 1 : 0;
      const bn = newest.has(b.id.toLowerCase()) ? 1 : 0;
      if (an !== bn) return bn - an;
      return cmpVersion(a, b);
    });

    $("idmeta").textContent = `${picked.length} unique`;

    if (filter === "all") {
      const cap = picked.slice(0, 400);
      $("labs").innerHTML = `<ul class="ids">${cap.map((x) => itemHtml(x, newest)).join("")}</ul>${
        picked.length > 400 ? `<p class="meta">Showing 400 of ${picked.length}. Search to narrow.</p>` : ""
      }`;
      return;
    }

    const groups = filter === "frontier" ? FRONTIER : [filter];
    $("labs").innerHTML = groups.map((lab) => {
      const items = picked.filter((x) => x.lab === lab);
      if (!items.length) return "";
      return `<h3>${esc(LABELS[lab] || lab)} <span class="meta">${items.length}</span></h3>
        <ul class="ids">${items.map((x) => itemHtml(x, newest)).join("")}</ul>`;
    }).join("") || `<p class="meta">No matching IDs.</p>`;
  };

  $("filters").addEventListener("click", (e) => {
    const btn = e.target.closest("button[data-filter]");
    if (!btn) return;
    filter = btn.dataset.filter;
    for (const b of $("filters").querySelectorAll("button")) b.classList.toggle("on", b === btn);
    render();
  });
  $("q").addEventListener("input", render);
  render();
}

load().catch((err) => {
  $("stamp").textContent = String(err);
});
