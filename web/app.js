const $ = (id) => document.getElementById(id);

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
  const events = s.events || [];
  $("events").innerHTML = events.length
    ? events.map((e) => `<li class="${e.op === "remove" ? "remove" : "add"}"><span class="meta">${esc(fmtTime(e.ts))} ${esc(e.source)}</span> ${e.op === "add" ? "+" : "−"} ${esc(e.id)}</li>`).join("")
    : `<li class="meta">${s.baseline ? "Baseline scan — next diff will show adds/removes." : "No changes yet."}</li>`;

  const all = [];
  const ids = s.ids || {};
  for (const [src, list] of Object.entries(ids)) {
    for (const id of list) all.push({ src, id });
  }
  all.sort((a, b) => a.id.localeCompare(b.id));
  const render = (q) => {
    const needle = (q || "").trim().toLowerCase();
    const shown = needle ? all.filter((x) => x.id.toLowerCase().includes(needle) || x.src.includes(needle)) : all;
    $("idmeta").textContent = `${shown.length} / ${all.length}`;
    $("ids").innerHTML = shown.slice(0, 400).map((x) => `<li><span class="meta">${esc(x.src)}</span> ${esc(x.id)}</li>`).join("");
  };
  $("q").addEventListener("input", (e) => render(e.target.value));
  render("");
}

load().catch((err) => {
  $("stamp").textContent = String(err);
});
