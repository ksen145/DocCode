//! Рендер единого интерактивного `index.html`.
//!
//! CSS и JS встроены в файл: документацию можно открыть локально без сервера
//! и без интернета. Markdown от LLM преобразуется в безопасный HTML небольшим
//! собственным рендерером (без внешних зависимостей); исходный код вставляется
//! только после экранирования.

use crate::docs::site::SiteDoc;
use std::fmt::Write;

/// Рендер всего сайта.
pub fn render_site(doc: &SiteDoc) -> String {
    let data = serde_json::to_string(doc).unwrap_or_else(|_| "{}".to_string());
    let data = json_for_script(&data);
    let title = escape_html(&doc.project);
    let generated = if doc.generated_at.is_empty() {
        String::new()
    } else {
        format!(
            "<span class=\"generated\" data-generated=\"{}\"></span>",
            escape_html(&doc.generated_at)
        )
    };
    let overview = if doc.overview_html.is_empty() {
        "<p class=\"muted\">Обзор проекта не сгенерирован.</p>".to_string()
    } else {
        doc.overview_html.clone()
    };
    let top_components = doc
        .components
        .iter()
        .map(|component| {
            format!(
                "<a class=\"fallback-card\" href=\"#/component/{}\"><strong>{}</strong><span>{} файлов</span></a>",
                escape_html(&component.id),
                escape_html(&component.name),
                component.all_files().len()
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let mut html = String::with_capacity(data.len() + 32_768);
    html.push_str(
        r##"<!DOCTYPE html>
<html lang="ru" data-theme="light">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<meta name="generator" content="DocCode">
<title>"##,
    );
    html.push_str(&title);
    html.push_str(
        r##" — документация</title>
<script>
try {
  var saved = localStorage.getItem('doccode-theme');
  if (saved) document.documentElement.dataset.theme = saved;
  else if (window.matchMedia && matchMedia('(prefers-color-scheme: dark)').matches) {
    document.documentElement.dataset.theme = 'dark';
  }
} catch (e) {}
</script>
<style>
:root {
  --bg: #f4f6fb;
  --panel: #ffffff;
  --panel-2: #f8fafc;
  --text: #172033;
  --muted: #667085;
  --border: #e3e7ef;
  --accent: #3451d1;
  --accent-2: #6d5bd0;
  --accent-soft: #eef2ff;
  --code-bg: #0f172a;
  --code-text: #e2e8f0;
  --ok: #07875f;
  --ok-soft: #e7f8f0;
  --warn: #a15c00;
  --warn-soft: #fff4df;
  --bad: #b42318;
  --bad-soft: #fff0ee;
  --gray-soft: #eef1f6;
  --shadow: 0 1px 2px rgba(16, 24, 40, .05), 0 12px 32px rgba(16, 24, 40, .06);
  --radius: 14px;
}
[data-theme="dark"] {
  --bg: #0c111d;
  --panel: #111827;
  --panel-2: #151e31;
  --text: #e6eaf2;
  --muted: #98a2b3;
  --border: #263149;
  --accent: #8ea2ff;
  --accent-2: #c4b5fd;
  --accent-soft: #1e2a4a;
  --code-bg: #070c16;
  --code-text: #dbe5f5;
  --ok: #3ddc97;
  --ok-soft: #0d2f24;
  --warn: #f5b84b;
  --warn-soft: #33260e;
  --bad: #ff8a80;
  --bad-soft: #3a1715;
  --gray-soft: #1b2437;
  --shadow: 0 1px 2px rgba(0, 0, 0, .3), 0 18px 48px rgba(0, 0, 0, .35);
}
* { box-sizing: border-box; }
html, body { height: 100%; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font: 15px/1.6 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
}
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
code, pre, .mono { font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace; }
.topbar {
  position: sticky; top: 0; z-index: 30;
  height: 58px; display: flex; align-items: center; gap: 18px;
  padding: 0 22px; background: color-mix(in srgb, var(--panel) 92%, transparent);
  border-bottom: 1px solid var(--border); backdrop-filter: blur(12px);
}
.brand { display: flex; align-items: center; gap: 10px; min-width: 220px; }
.brand-mark {
  width: 30px; height: 30px; border-radius: 9px; display: grid; place-items: center;
  background: linear-gradient(135deg, var(--accent), var(--accent-2)); color: #fff;
  font-weight: 800; font-size: 14px; box-shadow: 0 4px 12px rgba(52, 81, 209, .3);
}
.brand-name { font-weight: 750; letter-spacing: -.01em; }
.brand-sub { color: var(--muted); font-size: 12px; margin-left: 2px; }
.search-wrap { flex: 1; max-width: 620px; position: relative; }
.search-wrap::before {
  content: "⌕"; position: absolute; left: 12px; top: 7px; color: var(--muted); font-size: 17px;
}
#search {
  width: 100%; height: 38px; padding: 0 14px 0 34px; border-radius: 10px;
  border: 1px solid var(--border); background: var(--panel-2); color: var(--text);
  outline: none; font-size: 14px;
}
#search:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-soft); }
.actions { display: flex; align-items: center; gap: 8px; margin-left: auto; }
.button, .icon-button {
  border: 1px solid var(--border); background: var(--panel); color: var(--text);
  border-radius: 9px; cursor: pointer; font: inherit; font-size: 13px;
}
.button { padding: 7px 11px; }
.icon-button { width: 34px; height: 34px; display: grid; place-items: center; font-size: 16px; }
.button:hover, .icon-button:hover { border-color: var(--accent); color: var(--accent); }
.layout {
  display: grid; grid-template-columns: 326px minmax(0, 1fr);
  height: calc(100vh - 58px);
}
.sidebar {
  overflow: auto; padding: 16px 12px 40px; background: var(--panel);
  border-right: 1px solid var(--border);
}
.main-nav { display: grid; grid-template-columns: 1fr 1fr; gap: 7px; margin-bottom: 14px; }
.main-nav button {
  border: 1px solid var(--border); background: var(--panel-2); color: var(--text);
  border-radius: 9px; padding: 8px 10px; cursor: pointer; font: inherit; font-size: 13px;
}
.main-nav button:hover, .main-nav button.active { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }
.tabs { display: flex; align-items: center; gap: 6px; margin: 6px 0 10px; }
.tab {
  border: 0; background: transparent; color: var(--muted); font: inherit; font-size: 12px;
  font-weight: 700; text-transform: uppercase; letter-spacing: .05em; padding: 5px 6px;
  border-radius: 7px; cursor: pointer;
}
.tab.active { color: var(--accent); background: var(--accent-soft); }
.tree-actions { margin-left: auto; display: flex; gap: 2px; }
.tree-actions button { border: 0; background: transparent; color: var(--muted); cursor: pointer; font-size: 15px; }
.tree details { margin: 0; }
.tree summary, .tree-row {
  display: flex; align-items: center; gap: 7px; padding: 5px 6px; border-radius: 7px;
  list-style: none; cursor: pointer; min-height: 30px;
}
.tree summary::-webkit-details-marker { display: none; }
.tree summary:hover, .tree-row:hover { background: var(--panel-2); }
.tree summary::before {
  content: "›"; display: inline-block; width: 12px; color: var(--muted);
  transition: transform .15s ease; flex: none;
}
.tree details[open] > summary::before { transform: rotate(90deg); }
.tree .leaf { padding-left: 25px; }
.tree-link { color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
.tree-link:hover { color: var(--accent); text-decoration: none; }
.tree-link.active { color: var(--accent); font-weight: 700; }
.tree .count { color: var(--muted); font-size: 11px; flex: none; }
.tree .kind { color: var(--muted); font-size: 10px; text-transform: uppercase; flex: none; }
.dot { width: 7px; height: 7px; border-radius: 50%; flex: none; background: var(--muted); }
.dot.verified { background: var(--ok); }
.dot.review { background: var(--warn); }
.dot.failed { background: var(--bad); }
.dot.unverified, .dot.pending { background: var(--muted); }
.content { overflow: auto; padding: 34px 46px 90px; }
.page { max-width: 1060px; margin: 0 auto; }
.hero { display: flex; align-items: flex-start; justify-content: space-between; gap: 24px; margin-bottom: 22px; }
.hero h1 { margin: 0 0 8px; font-size: 34px; line-height: 1.15; letter-spacing: -.03em; }
.hero p { margin: 0; color: var(--muted); max-width: 720px; }
.status-badge {
  display: inline-flex; align-items: center; gap: 6px; border-radius: 999px;
  padding: 5px 10px; font-size: 12px; font-weight: 700; white-space: nowrap;
  border: 1px solid transparent;
}
.status-badge.verified { color: var(--ok); background: var(--ok-soft); border-color: color-mix(in srgb, var(--ok) 25%, transparent); }
.status-badge.review { color: var(--warn); background: var(--warn-soft); border-color: color-mix(in srgb, var(--warn) 25%, transparent); }
.status-badge.failed { color: var(--bad); background: var(--bad-soft); border-color: color-mix(in srgb, var(--bad) 25%, transparent); }
.status-badge.unverified, .status-badge.pending { color: var(--muted); background: var(--gray-soft); border-color: var(--border); }
.stats { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; margin: 18px 0 26px; }
.stat { background: var(--panel); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px 15px; box-shadow: var(--shadow); }
.stat b { display: block; font-size: 24px; line-height: 1.2; letter-spacing: -.03em; }
.stat span { color: var(--muted); font-size: 12px; }
.map-section { background: var(--panel); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; margin: 24px 0; box-shadow: var(--shadow); }
.map-section h2 { margin: 0 0 4px; font-size: 19px; }
.map-section .hint { color: var(--muted); font-size: 13px; margin: 0 0 16px; }
.map { display: grid; grid-template-columns: minmax(190px, .75fr) 34px minmax(0, 1.6fr); gap: 14px; align-items: stretch; }
.map-entry, .map-components { min-width: 0; }
.map-label { font-size: 11px; letter-spacing: .06em; text-transform: uppercase; color: var(--muted); font-weight: 750; margin-bottom: 7px; }
.entry-card {
  display: block; border: 1px solid var(--border); border-radius: 10px; padding: 10px 12px;
  background: var(--panel-2); color: var(--text); margin-bottom: 7px;
}
.entry-card:hover { border-color: var(--accent); text-decoration: none; }
.entry-card b { display: block; font-size: 13px; }
.entry-card span { color: var(--muted); font-size: 11px; }
.map-arrow { display: grid; place-items: center; color: var(--accent); font-size: 26px; font-weight: 300; }
.component-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
.component-card {
  border: 1px solid var(--border); background: var(--panel-2); border-radius: 10px;
  padding: 10px 12px; color: var(--text); display: block; min-height: 66px;
}
.component-card:hover { border-color: var(--accent); text-decoration: none; }
.component-card b { display: block; font-size: 13px; margin-bottom: 2px; }
.component-card span { color: var(--muted); font-size: 11px; }
.fallback-card { display: flex; justify-content: space-between; padding: 10px; border: 1px solid var(--border); border-radius: 9px; margin: 6px 0; color: var(--text); }
.fallback-card span { color: var(--muted); font-size: 12px; }
.breadcrumbs { display: flex; gap: 6px; align-items: center; color: var(--muted); font-size: 12px; margin-bottom: 12px; flex-wrap: wrap; }
.breadcrumbs a { color: var(--muted); }
.breadcrumbs .sep { opacity: .5; }
.page-title { margin: 0 0 8px; font-size: 31px; line-height: 1.2; letter-spacing: -.03em; }
.page-lead { color: var(--muted); margin: 0 0 14px; max-width: 820px; }
.meta-row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin: 0 0 22px; }
.pill { display: inline-flex; gap: 6px; padding: 4px 9px; border-radius: 999px; background: var(--gray-soft); color: var(--muted); font-size: 12px; }
.verify-line { border: 1px solid var(--border); background: var(--panel); border-radius: 11px; padding: 10px 12px; color: var(--muted); font-size: 12px; margin: 0 0 20px; }
.verify-line b { color: var(--text); }
.markdown { font-size: 15px; line-height: 1.72; overflow-wrap: anywhere; }
.markdown h1 { font-size: 30px; line-height: 1.2; letter-spacing: -.03em; margin: 0 0 16px; }
.markdown h2 { font-size: 22px; line-height: 1.25; letter-spacing: -.02em; margin: 32px 0 10px; padding-top: 6px; border-top: 1px solid var(--border); }
.markdown h3 { font-size: 17px; margin: 24px 0 8px; }
.markdown h4 { font-size: 15px; margin: 20px 0 6px; }
.markdown p { margin: 9px 0; }
.markdown ul, .markdown ol { margin: 9px 0; padding-left: 24px; }
.markdown li { margin: 4px 0; }
.markdown code { background: var(--gray-soft); border: 1px solid var(--border); border-radius: 5px; padding: 1px 5px; font-size: .9em; }
.markdown pre { background: var(--code-bg); color: var(--code-text); border-radius: 11px; padding: 15px 17px; overflow: auto; border: 1px solid var(--border); margin: 14px 0; }
.markdown pre code { background: transparent; border: 0; padding: 0; color: inherit; font-size: 12.5px; line-height: 1.55; }
.markdown blockquote { border-left: 3px solid var(--accent); margin: 14px 0; padding: 4px 0 4px 14px; color: var(--muted); }
.markdown table { border-collapse: collapse; width: 100%; margin: 14px 0; font-size: 13px; }
.markdown th, .markdown td { border: 1px solid var(--border); padding: 7px 9px; text-align: left; vertical-align: top; }
.markdown th { background: var(--panel-2); }
.markdown hr { border: 0; border-top: 1px solid var(--border); margin: 24px 0; }
.markdown a { text-decoration: underline; text-underline-offset: 2px; }
.files-block { margin-top: 30px; }
.files-block h2 { font-size: 17px; margin: 0 0 10px; }
.file-chip { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border: 1px solid var(--border); border-radius: 8px; margin: 5px 0; background: var(--panel); }
.file-chip a { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.file-chip .muted { font-size: 11px; }
.entity { border: 1px solid var(--border); border-radius: 10px; margin: 9px 0; background: var(--panel); overflow: hidden; }
.entity summary { padding: 10px 12px; cursor: pointer; display: flex; align-items: center; gap: 9px; list-style: none; }
.entity summary::-webkit-details-marker { display: none; }
.entity summary:hover { background: var(--panel-2); }
.entity .kind { font-size: 10px; font-weight: 750; text-transform: uppercase; color: var(--accent); background: var(--accent-soft); padding: 2px 6px; border-radius: 5px; }
.entity .name { font-weight: 650; }
.entity .loc { color: var(--muted); font-size: 11px; margin-left: auto; }
.code-tools { display: flex; gap: 6px; flex-wrap: wrap; padding: 0 12px 10px; }
.code-tools .button { font-size: 11px; padding: 5px 8px; }
.entity pre { margin: 0; border-radius: 0; border-left: 0; border-right: 0; border-bottom: 1px solid var(--border); max-height: 520px; }
.warnings { background: var(--warn-soft); border: 1px solid color-mix(in srgb, var(--warn) 30%, transparent); border-radius: 11px; padding: 12px 14px; margin: 16px 0; }
.warnings h3 { margin: 0 0 6px; font-size: 13px; color: var(--warn); }
.warnings ul { margin: 0; padding-left: 20px; font-size: 12px; color: var(--text); }
.quality-table { width: 100%; border-collapse: collapse; background: var(--panel); border: 1px solid var(--border); border-radius: 11px; overflow: hidden; }
.quality-table th, .quality-table td { padding: 9px 11px; border-bottom: 1px solid var(--border); text-align: left; font-size: 12px; vertical-align: top; }
.quality-table th { background: var(--panel-2); color: var(--muted); font-size: 11px; text-transform: uppercase; letter-spacing: .04em; }
.quality-table tr:last-child td { border-bottom: 0; }
.muted { color: var(--muted); }
.empty { border: 1px dashed var(--border); border-radius: 12px; padding: 34px; text-align: center; color: var(--muted); }
.toast {
  position: fixed; right: 20px; bottom: 22px; z-index: 100; background: var(--text); color: var(--bg);
  border-radius: 9px; padding: 9px 13px; font-size: 12px; opacity: 0; transform: translateY(8px);
  transition: .2s ease; pointer-events: none;
}
.toast.show { opacity: 1; transform: translateY(0); }
.generated::before { content: "сгенерировано "; }
@media (max-width: 1000px) {
  .stats { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .content { padding: 26px 24px 70px; }
}
@media (max-width: 880px) {
  .topbar { height: auto; min-height: 58px; padding: 9px 14px; flex-wrap: wrap; gap: 10px; }
  .brand { min-width: 0; }
  .brand-sub { display: none; }
  .search-wrap { order: 4; flex-basis: 100%; max-width: none; }
  .layout { display: block; height: auto; }
  .sidebar { max-height: 46vh; border-right: 0; border-bottom: 1px solid var(--border); }
  .content { overflow: visible; }
  .hero { display: block; }
  .hero h1 { font-size: 28px; }
  .map { grid-template-columns: 1fr; }
  .map-arrow { transform: rotate(90deg); }
  .component-grid { grid-template-columns: 1fr; }
}
@media (max-width: 560px) {
  .stats { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .content { padding: 20px 14px 60px; }
  .status-badge { margin-top: 8px; }
}
</style>
</head>
<body>
<header class="topbar">
  <div class="brand">
    <div class="brand-mark">DC</div>
    <div>
      <div class="brand-name">DocCode</div>
      <div class="brand-sub">документация по коду</div>
    </div>
  </div>
  <div class="search-wrap">
    <input id="search" type="search" placeholder="Поиск по компонентам, файлам и сущностям… ( / )" autocomplete="off" spellcheck="false">
  </div>
  <div class="actions">
    <button class="icon-button" id="theme-toggle" title="Переключить тему" aria-label="Переключить тему">◐</button>
  </div>
</header>
<div class="layout">
  <aside class="sidebar">
    <nav class="main-nav">
      <button data-route="#/overview" class="active">Обзор проекта</button>
      <button data-route="#/quality">Качество D1</button>
    </nav>
    <div class="tabs">
      <button class="tab active" data-tab="components">Компоненты</button>
      <button class="tab" data-tab="files">Файлы</button>
      <div class="tree-actions">
        <button id="expand-all" title="Развернуть всё">＋</button>
        <button id="collapse-all" title="Свернуть всё">−</button>
      </div>
    </div>
    <div id="tree" class="tree"></div>
  </aside>
  <main id="content" class="content">
    <div class="page">
      <div class="hero">
        <div>
          <h1>"##,
    );
    html.push_str(&title);
    html.push_str(
        r##"</h1>
          <p>Интерактивная документация, собранная по векторному индексу кода. Выберите компонент слева или начните с обзора.</p>
        </div>
        <span class="status-badge "##,
    );
    html.push_str(doc.overview_status.css());
    html.push_str(r##"">"##);
    html.push_str(doc.overview_status.label());
    html.push_str(
        r##"</span>
      </div>
      <div id="overview-server" class="markdown">"##,
    );
    html.push_str(&overview);
    html.push_str(
        r##"</div>
      <div class="files-block">
        <h2>Верхний уровень</h2>
        <div class="component-grid">"##,
    );
    html.push_str(&top_components);
    html.push_str(
        r##"</div>
      </div>
      <p class="muted" style="margin-top:26px">"##,
    );
    html.push_str(&generated);
    html.push_str(
        r##"</p>
    </div>
  </main>
</div>
<div id="toast" class="toast" role="status" aria-live="polite"></div>
<script>
const DATA = "##,
    );
    html.push_str(&data);
    html.push_str(
        r##";
(function () {
  "use strict";
  const $ = (selector, root) => (root || document).querySelector(selector);
  const $$ = (selector, root) => Array.from((root || document).querySelectorAll(selector));
  const content = $('#content');
  const tree = $('#tree');
  const search = $('#search');
  const toast = $('#toast');
  const statusLabels = {
    verified: 'Проверено D1',
    review: 'Требует проверки',
    unverified: 'Не проверено',
    failed: 'Ошибка',
    pending: 'Ожидает'
  };

  function esc(value) {
    return String(value == null ? '' : value)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
  }
  function statusClass(value) { return value === 'needs_review' ? 'review' : (value || 'pending'); }
  function statusBadge(value) {
    const key = statusClass(value);
    return '<span class="status-badge ' + key + '">' + esc(statusLabels[key] || key) + '</span>';
  }
  function dot(value) { return '<span class="dot ' + statusClass(value) + '"></span>'; }
  function pass(value, suffix) { return value == null ? '—' : String(value) + (suffix || ''); }
  function componentHref(id) { return '#/component/' + encodeURIComponent(id); }
  function fileHref(path) { return '#/file/' + encodeURIComponent(path); }
  function allFiles(component) {
    const files = [];
    const seen = new Set();
    (function walk(node) {
      for (const file of node.files || []) {
        if (!seen.has(file)) { seen.add(file); files.push(file); }
      }
      for (const child of node.children || []) walk(child);
    })(component);
    return files;
  }
  function vsCodeHref(path, line) {
    const root = String(DATA.root || '').replace(/\\/g, '/').replace(/\/+$/, '');
    if (!root) return '';
    return 'vscode://file/' + encodeURI(root + '/' + path).replace(/#/g, '%23') + (line ? ':' + line : '');
  }
  function notify(text) {
    toast.textContent = text;
    toast.classList.add('show');
    clearTimeout(notify.timer);
    notify.timer = setTimeout(function () { toast.classList.remove('show'); }, 2200);
  }
  async function copyText(text) {
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(text);
      } else {
        legacyCopy(text);
      }
      notify('Скопировано: ' + text);
    } catch (e) {
      try {
        legacyCopy(text);
        notify('Скопировано: ' + text);
      } catch (fallbackError) {
        notify('Не удалось скопировать');
      }
    }
  }
  function legacyCopy(text) {
    const area = document.createElement('textarea');
    area.value = text;
    area.setAttribute('readonly', '');
    area.style.position = 'fixed';
    area.style.opacity = '0';
    document.body.appendChild(area);
    area.select();
    const copied = document.execCommand('copy');
    document.body.removeChild(area);
    if (!copied) throw new Error('copy failed');
  }

  // Плоский список и родители для хлебных крошек.
  const flat = [];
  const parentOf = new Map();
  function flatten(list, parent) {
    for (const item of list || []) {
      flat.push(item);
      parentOf.set(item.id, parent || null);
      flatten(item.children, item);
    }
  }
  flatten(DATA.components, null);
  const byId = new Map(flat.map(item => [item.id, item]));
  const fileByPath = new Map((DATA.files || []).map(file => [file.path, file]));

  function statCards() {
    const s = DATA.stats || {};
    const items = [
      [s.files, 'файлов'],
      [s.entities, 'сущностей'],
      [s.components, 'компонентов'],
      [s.chunks, 'векторных фрагментов'],
      [s.verified, 'проверено D1'],
      [s.needs_review, 'на проверке D1']
    ];
    return '<div class="stats">' + items.map(item =>
      '<div class="stat"><b>' + esc(item[0] || 0) + '</b><span>' + esc(item[1]) + '</span></div>'
    ).join('') + '</div>';
  }

  function projectMap() {
    const entries = (DATA.entry_points || []).map(entry =>
      '<a class="entry-card" href="' + fileHref(entry.path) + '">' +
        '<b>' + esc(entry.path) + ':' + esc(entry.line) + '</b>' +
        '<span>' + esc(entry.description || 'точка запуска') + '</span>' +
      '</a>'
    ).join('') || '<div class="entry-card"><b>Не найдена</b><span>начните с файлов в корне</span></div>';
    const components = (DATA.components || []).map(component => {
      const children = (component.children || []).map(child => esc(child.name)).join(', ');
      return '<a class="component-card" href="' + componentHref(component.id) + '">' +
        '<b>' + esc(component.name) + '</b>' +
        '<span>' + esc(allFiles(component).length) + ' файлов' + (children ? ' · ' + children : '') + '</span>' +
      '</a>';
    }).join('') || '<div class="muted">Компоненты не выделены.</div>';

    return '<section class="map-section">' +
      '<h2>Карта проекта для разработчика</h2>' +
      '<p class="hint">Слева — точка запуска программы, справа — логические компоненты. Нажмите на карточку, чтобы перейти.</p>' +
      '<div class="map">' +
        '<div class="map-entry"><div class="map-label">Точка входа</div>' + entries + '</div>' +
        '<div class="map-arrow">→</div>' +
        '<div class="map-components"><div class="map-label">Компоненты</div><div class="component-grid">' + components + '</div></div>' +
      '</div>' +
    '</section>';
  }

  function warningsBlock() {
    if (!DATA.warnings || !DATA.warnings.length) return '';
    return '<div class="warnings"><h3>Замечания генерации</h3><ul>' +
      DATA.warnings.map(item => '<li>' + esc(item) + '</li>').join('') + '</ul></div>';
  }

  function renderOverview() {
    const s = DATA.stats || {};
    const badge = statusBadge(DATA.overview_status);
    const verification = DATA.overview_verification
      ? '<div class="verify-line"><b>' + esc(DATA.overview_verification.note) + '</b></div>' : '';
    content.innerHTML = '<div class="page">' +
      '<div class="hero"><div><h1>' + esc(DATA.project) + '</h1>' +
        '<p>Векторный индекс: ' + esc(s.index_backend || '—') + ' · ' + esc(s.chunks || 0) + ' фрагментов кода · ' + esc(s.files || 0) + ' файлов</p>' +
      '</div>' + badge + '</div>' +
      statCards() + verification + projectMap() +
      '<article class="markdown">' + (DATA.overview_html || '<p class="muted">Обзор не сгенерирован.</p>') + '</article>' +
      warningsBlock() +
      (DATA.generated_at ? '<p class="muted" style="margin-top:26px">Сгенерировано: ' + esc(new Date(Number(DATA.generated_at) * 1000).toLocaleString()) + '</p>' : '') +
    '</div>';
    window.scrollTo(0, 0);
  }

  function breadcrumbs(parent, current) {
    const links = [];
    let node = parent;
    while (node) {
      links.unshift('<a href="' + componentHref(node.id) + '">' + esc(node.name) + '</a>');
      node = parentOf.get(node.id);
    }
    links.push('<span>' + esc(current) + '</span>');
    return '<div class="breadcrumbs"><a href="#/overview">Обзор</a><span class="sep">/</span>' + links.join('<span class="sep">/</span>') + '</div>';
  }

  function filesBlock(component) {
    const files = allFiles(component);
    if (!files.length) return '';
    return '<div class="files-block"><h2>Файлы компонента</h2>' + files.map(path => {
      const file = fileByPath.get(path);
      const count = file && file.entities ? file.entities.length : 0;
      return '<div class="file-chip"><a href="' + fileHref(path) + '">' + esc(path) + '</a>' +
        '<span class="muted">' + count + ' сущностей</span></div>';
    }).join('') + '</div>';
  }

  function renderComponent(id) {
    const component = byId.get(id);
    if (!component) { renderNotFound('Компонент не найден'); return; }
    const verification = component.verification
      ? '<div class="verify-line"><b>' + esc(component.verification.note) + '</b></div>' : '';
    const parent = parentOf.get(component.id);
    content.innerHTML = '<div class="page">' +
      breadcrumbs(parent, component.name) +
      '<h1 class="page-title">' + esc(component.name) + '</h1>' +
      '<p class="page-lead">' + esc(component.description || '') + '</p>' +
      '<div class="meta-row">' + statusBadge(component.status) +
        '<span class="pill">' + allFiles(component).length + ' файлов</span>' +
      '</div>' + verification +
      '<article class="markdown">' + (component.html || '<p class="muted">Описание компонента ещё не сгенерировано.</p>') + '</article>' +
      filesBlock(component) +
      warningsBlock() +
    '</div>';
    window.scrollTo(0, 0);
  }

  function renderFile(path) {
    const file = fileByPath.get(path);
    if (!file) { renderNotFound('Файл не найден'); return; }
    const component = file.component ? byId.get(file.component) : null;
    const entities = (file.entities || []).map((entity, index) => {
      const location = file.path + ':' + entity.line;
      const code = entity.code ? '<pre><code>' + esc(entity.code) + '</code></pre>' : '<p class="muted" style="padding:0 12px 12px">Исходный код не сохранён.</p>';
      const vs = vsCodeHref(file.path, entity.line);
      return '<details class="entity" ' + (index === 0 ? 'open' : '') + '>' +
        '<summary><span class="kind">' + esc(entity.kind) + '</span>' +
          '<span class="name">' + esc(entity.name) + '</span>' +
          '<span class="loc">' + esc(location) + '</span></summary>' +
        '<div class="code-tools">' +
          '<button class="button" data-copy="' + esc(location) + '">Копировать path:line</button>' +
          (vs ? '<a class="button" href="' + esc(vs) + '">Открыть в VS Code</a>' : '') +
        '</div>' + code +
      '</details>';
    }).join('');
    const vs = vsCodeHref(file.path, 1);
    content.innerHTML = '<div class="page">' +
      '<div class="breadcrumbs"><a href="#/overview">Обзор</a><span class="sep">/</span>' +
        (component ? '<a href="' + componentHref(component.id) + '">' + esc(component.name) + '</a><span class="sep">/</span>' : '') +
        '<span>' + esc(file.path) + '</span></div>' +
      '<h1 class="page-title mono" style="font-size:23px">' + esc(file.path) + '</h1>' +
      '<div class="meta-row"><span class="pill">' + esc(file.language) + '</span>' +
        '<span class="pill">' + (file.entities || []).length + ' сущностей</span>' +
        (vs ? '<a class="button" href="' + esc(vs) + '">Открыть в VS Code</a>' : '') +
      '</div>' +
      (entities || '<div class="empty">Из этого файла не извлечено ни одной сущности. Откройте файл в редакторе по пути <code>' + esc(file.path) + '</code>.</div>') +
    '</div>';
    window.scrollTo(0, 0);
  }

  function renderQuality() {
    const rows = [];
    rows.push('<tr><td><b>Обзор проекта</b></td><td>' + statusBadge(DATA.overview_status) + '</td>' +
      '<td>' + pass(DATA.overview_verification && DATA.overview_verification.factual) + '</td>' +
      '<td>' + pass(DATA.overview_verification && DATA.overview_verification.completeness, '/5') + '</td>' +
      '<td>' + esc((DATA.overview_verification && DATA.overview_verification.note) || '') + '</td></tr>');
    for (const component of flat) {
      const v = component.verification || {};
      rows.push('<tr><td><a href="' + componentHref(component.id) + '">' + esc(component.name) + '</a></td>' +
        '<td>' + statusBadge(component.status) + '</td>' +
        '<td>' + pass(v.factual) + '</td><td>' + pass(v.completeness, '/5') + '</td>' +
        '<td>' + esc(v.note || '') + '</td></tr>');
    }
    const minFactuality = Number(DATA.d1_min_factuality || 0.85).toFixed(2);
    const minCompleteness = Number(DATA.d1_min_completeness || 3);
    content.innerHTML = '<div class="page"><h1 class="page-title">Качество документации</h1>' +
      '<p class="page-lead">Модель D1 проверяет каждый документ на фактическую точность и полноту. ' +
      'Текущий порог: точность ≥ ' + minFactuality + ', полнота ≥ ' + minCompleteness + '/5. ' +
      'Документы ниже порога помечены как «требует проверки»; при недоступности D1 — «не проверено».</p>' +
      statCards() +
      '<table class="quality-table"><thead><tr><th>Документ</th><th>Статус</th><th>Точность</th><th>Полнота</th><th>Комментарий D1</th></tr></thead><tbody>' +
      rows.join('') + '</tbody></table>' + warningsBlock() + '</div>';
    window.scrollTo(0, 0);
  }

  function renderNotFound(message) {
    content.innerHTML = '<div class="page"><div class="empty">' + esc(message) + '<br><a href="#/overview">Вернуться к обзору</a></div></div>';
  }

  function renderSearch(query) {
    const q = query.trim().toLowerCase();
    if (!q) { route(); return; }
    const results = [];
    for (const component of flat) {
      const haystack = [component.name, component.description, (component.files || []).join(' ')].join(' ').toLowerCase();
      if (haystack.includes(q)) {
        results.push('<a class="file-chip" href="' + componentHref(component.id) + '"><span class="kind">компонент</span><span style="flex:1">' + esc(component.name) + '</span></a>');
      }
    }
    for (const file of DATA.files || []) {
      const entities = (file.entities || []).map(entity => entity.name + ' ' + entity.kind).join(' ');
      const haystack = [file.path, entities].join(' ').toLowerCase();
      if (haystack.includes(q)) {
        results.push('<a class="file-chip" href="' + fileHref(file.path) + '"><span class="kind">файл</span><span style="flex:1">' + esc(file.path) + '</span><span class="muted">' + (file.entities || []).length + '</span></a>');
      }
      for (const entity of file.entities || []) {
        if ((entity.name + ' ' + entity.kind).toLowerCase().includes(q)) {
          results.push('<a class="file-chip" href="' + fileHref(file.path) + '"><span class="kind">' + esc(entity.kind) + '</span><span style="flex:1">' + esc(entity.name) + '</span><span class="muted">' + esc(file.path) + ':' + entity.line + '</span></a>');
        }
      }
    }
    content.innerHTML = '<div class="page"><h1 class="page-title">Поиск: ' + esc(query) + '</h1>' +
      '<p class="page-lead">Найдено: ' + results.length + ' (первые 80)</p>' +
      (results.slice(0, 80).join('') || '<div class="empty">Ничего не найдено.</div>') + '</div>';
  }

  function componentLeaf(component) {
    const active = location.hash === componentHref(component.id) ? ' active' : '';
    return '<a class="tree-link' + active + '" href="' + componentHref(component.id) + '" title="' + esc(component.description) + '">' + esc(component.name) + '</a>' + dot(component.status);
  }
  function componentNode(component, depth) {
    const children = component.children || [];
    if (!children.length) {
      return '<div class="tree-row leaf">' + componentLeaf(component) + '</div>';
    }
    return '<details ' + (depth < 3 ? 'open' : '') + '><summary>' + componentLeaf(component) +
      '<span class="count">' + children.length + '</span></summary>' +
      children.map(child => componentNode(child, depth + 1)).join('') + '</details>';
  }
  function renderComponentTree() {
    tree.innerHTML = (DATA.components || []).map(component => componentNode(component, 0)).join('') ||
      '<div class="empty">Компоненты не выделены.</div>';
  }

  function buildFileTree(files) {
    const root = { dirs: new Map(), files: [] };
    for (const file of files) {
      const parts = file.path.split('/');
      let node = root;
      for (let i = 0; i < parts.length - 1; i++) {
        if (!node.dirs.has(parts[i])) node.dirs.set(parts[i], { dirs: new Map(), files: [] });
        node = node.dirs.get(parts[i]);
      }
      node.files.push(file);
    }
    return root;
  }
  function fileNode(node, name, depth) {
    const dirs = Array.from(node.dirs.entries()).sort((a, b) => a[0].localeCompare(b[0]));
    const files = node.files.slice().sort((a, b) => a.path.localeCompare(b.path));
    if (name) {
      return '<details ' + (depth < 2 ? 'open' : '') + '><summary><span class="tree-link">' + esc(name) + '</span></summary>' +
        dirs.map(entry => fileNode(entry[1], entry[0], depth + 1)).join('') +
        files.map(file => '<div class="tree-row leaf"><a class="tree-link' + (location.hash === fileHref(file.path) ? ' active' : '') + '" href="' + fileHref(file.path) + '">' + esc(file.path.split('/').pop()) + '</a><span class="count">' + (file.entities || []).length + '</span></div>').join('') +
      '</details>';
    }
    return dirs.map(entry => fileNode(entry[1], entry[0], depth + 1)).join('') +
      files.map(file => '<div class="tree-row"><a class="tree-link' + (location.hash === fileHref(file.path) ? ' active' : '') + '" href="' + fileHref(file.path) + '">' + esc(file.path) + '</a><span class="count">' + (file.entities || []).length + '</span></div>').join('');
  }
  function renderFileTree() {
    tree.innerHTML = fileNode(buildFileTree(DATA.files || []), '', 0) || '<div class="empty">Файлы не найдены.</div>';
  }

  let currentTab = 'components';
  function switchTab(tab) {
    currentTab = tab;
    $$('.tab').forEach(button => button.classList.toggle('active', button.dataset.tab === tab));
    if (tab === 'files') renderFileTree(); else renderComponentTree();
  }

  function setActiveNav() {
    const hash = location.hash || '#/overview';
    $$('.main-nav button').forEach(button => {
      const route = button.dataset.route;
      button.classList.toggle('active', route === hash || (route === '#/overview' && (hash === '' || hash === '#/')));
    });
  }

  function route() {
    const hash = location.hash || '#/overview';
    setActiveNav();
    if (hash.startsWith('#/component/')) {
      renderComponent(decodeURIComponent(hash.slice('#/component/'.length)));
    } else if (hash.startsWith('#/file/')) {
      renderFile(decodeURIComponent(hash.slice('#/file/'.length)));
    } else if (hash === '#/quality') {
      renderQuality();
    } else {
      renderOverview();
    }
    if (search.value) search.value = '';
    if (currentTab === 'files') renderFileTree(); else renderComponentTree();
  }

  // События.
  window.addEventListener('hashchange', route);
  $('#theme-toggle').addEventListener('click', function () {
    const next = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = next;
    try { localStorage.setItem('doccode-theme', next); } catch (e) {}
  });
  $$('.main-nav button').forEach(button => button.addEventListener('click', function () {
    if (location.hash === button.dataset.route) route();
    else location.hash = button.dataset.route;
  }));
  $$('.tab').forEach(button => button.addEventListener('click', function () { switchTab(button.dataset.tab); }));
  $('#expand-all').addEventListener('click', function () { $$('#tree details').forEach(item => item.open = true); });
  $('#collapse-all').addEventListener('click', function () { $$('#tree details').forEach(item => item.open = false); });
  search.addEventListener('input', function () { renderSearch(search.value); });
  search.addEventListener('keydown', function (event) {
    if (event.key === 'Escape') { search.value = ''; route(); search.blur(); }
  });
  document.addEventListener('keydown', function (event) {
    if (event.key === '/' && document.activeElement !== search) {
      event.preventDefault(); search.focus();
    }
  });
  document.addEventListener('click', function (event) {
    const copy = event.target.closest('[data-copy]');
    if (copy) { event.preventDefault(); copyText(copy.dataset.copy); return; }
    // Повторный клик по текущему маршруту (например, из результатов поиска)
    // не меняет hash, поэтому принудительно перерисовываем страницу.
    const link = event.target.closest('a[href^="#/"]');
    if (link && link.getAttribute('href') === location.hash) {
      event.preventDefault();
      route();
    }
  });

  $$('.generated').forEach(function (element) {
    const value = Number(element.dataset.generated);
    if (value > 0) element.textContent = new Date(value * 1000).toLocaleString();
  });

  route();
})();
</script>
</body>
</html>
"##,
    );
    html
}

/// Экранирование текста для HTML.
pub fn escape_html(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// JSON, безопасный для вставки внутрь `<script>`.
fn json_for_script(json: &str) -> String {
    json.replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

/// Минимальный Markdown → HTML.
///
/// Поддерживаются заголовки, абзацы, списки, цитаты, таблицы, горизонтальные
/// линии, fenced code и inline-разметка. Всё, что не распознано, остаётся
/// экранированным текстом — LLM не может внедрить произвольный HTML.
pub fn markdown_to_html(markdown: &str) -> String {
    let normalized = markdown.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let mut out = String::new();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            index += 1;
            continue;
        }

        // Fenced code.
        if let Some(fence) = trimmed.strip_prefix("```") {
            let lang = fence.trim();
            index += 1;
            let mut code = String::new();
            while index < lines.len() && !lines[index].trim_start().starts_with("```") {
                code.push_str(lines[index]);
                code.push('\n');
                index += 1;
            }
            if index < lines.len() {
                index += 1;
            }
            let class = if lang.is_empty() {
                String::new()
            } else {
                format!(
                    " class=\"language-{}\"",
                    lang.chars()
                        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                        .collect::<String>()
                )
            };
            let _ = writeln!(
                out,
                "<pre><code{}>{}</code></pre>",
                class,
                escape_html(&code)
            );
            continue;
        }

        // Headings.
        if let Some((level, text)) = heading(trimmed) {
            let _ = writeln!(out, "<h{level}>{}</h{level}>", inline(text));
            index += 1;
            continue;
        }

        // Horizontal rule.
        if matches!(trimmed, "---" | "***" | "___") {
            out.push_str("<hr>\n");
            index += 1;
            continue;
        }

        // Tables.
        if trimmed.starts_with('|')
            && index + 1 < lines.len()
            && is_table_separator(lines[index + 1].trim())
        {
            index = render_table(&lines, index, &mut out);
            continue;
        }

        // Unordered list.
        if is_unordered_item(line) {
            out.push_str("<ul>\n");
            while index < lines.len() && is_unordered_item(lines[index]) {
                let item = lines[index]
                    .trim_start()
                    .trim_start_matches(['-', '*', '+'])
                    .trim();
                let _ = writeln!(out, "<li>{}</li>", inline(item));
                index += 1;
            }
            out.push_str("</ul>\n");
            continue;
        }

        // Ordered list.
        if let Some(item) = ordered_item(line) {
            out.push_str("<ol>\n");
            let _ = writeln!(out, "<li>{}</li>", inline(item));
            index += 1;
            while index < lines.len() {
                if let Some(item) = ordered_item(lines[index]) {
                    let _ = writeln!(out, "<li>{}</li>", inline(item));
                    index += 1;
                } else if lines[index].trim().is_empty() {
                    index += 1;
                } else {
                    break;
                }
            }
            out.push_str("</ol>\n");
            continue;
        }

        // Blockquote.
        if trimmed.starts_with('>') {
            let mut quote = String::new();
            while index < lines.len() && lines[index].trim_start().starts_with('>') {
                let text = lines[index].trim_start().trim_start_matches('>').trim();
                if !quote.is_empty() {
                    quote.push(' ');
                }
                quote.push_str(text);
                index += 1;
            }
            let _ = writeln!(out, "<blockquote>{}</blockquote>", inline(&quote));
            continue;
        }

        // Paragraph: until a blank line or the start of another block.
        let mut paragraph = String::new();
        while index < lines.len() {
            let current = lines[index];
            let current_trimmed = current.trim();
            if current_trimmed.is_empty()
                || current_trimmed.starts_with("```")
                || heading(current_trimmed).is_some()
                || is_unordered_item(current)
                || ordered_item(current).is_some()
                || current_trimmed.starts_with('>')
                || matches!(current_trimmed, "---" | "***" | "___")
                || (current_trimmed.starts_with('|')
                    && index + 1 < lines.len()
                    && is_table_separator(lines[index + 1].trim()))
            {
                break;
            }
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(current_trimmed);
            index += 1;
        }
        if !paragraph.is_empty() {
            let _ = writeln!(out, "<p>{}</p>", inline(&paragraph));
        } else {
            index += 1;
        }
    }

    out
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && line.chars().nth(hashes) == Some(' ') {
        Some((hashes, line[hashes..].trim()))
    } else {
        None
    }
}

fn is_unordered_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    let mut chars = trimmed.chars();
    matches!(chars.next(), Some('-') | Some('*') | Some('+')) && chars.next() == Some(' ')
}

fn ordered_item(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let digits = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || !trimmed[digits..].starts_with(". ") {
        return None;
    }
    Some(trimmed[digits + 2..].trim())
}

fn is_table_separator(line: &str) -> bool {
    line.starts_with('|')
        && line.ends_with('|')
        && line.trim_matches('|').split('|').all(|cell| {
            let cell = cell.trim();
            !cell.is_empty() && cell.chars().all(|c| matches!(c, '-' | ':' | ' '))
        })
}

fn table_cells(line: &str) -> Vec<&str> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect()
}

fn render_table(lines: &[&str], start: usize, out: &mut String) -> usize {
    let headers = table_cells(lines[start]);
    let mut index = start + 2;
    out.push_str("<table>\n<thead><tr>");
    for header in &headers {
        let _ = write!(out, "<th>{}</th>", inline(header));
    }
    out.push_str("</tr></thead>\n<tbody>\n");
    while index < lines.len() && lines[index].trim().starts_with('|') {
        out.push_str("<tr>");
        for cell in table_cells(lines[index]) {
            let _ = write!(out, "<td>{}</td>", inline(cell));
        }
        out.push_str("</tr>\n");
        index += 1;
    }
    out.push_str("</tbody>\n</table>\n");
    index
}

/// Inline-разметка. Входные данные сначала экранируются.
fn inline(text: &str) -> String {
    inline_escaped(&escape_html(text))
}

fn inline_escaped(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0usize;

    while index < chars.len() {
        match chars[index] {
            '`' => {
                if let Some(end) = find_char(&chars, index + 1, '`') {
                    out.push_str("<code>");
                    out.extend(&chars[index + 1..end]);
                    out.push_str("</code>");
                    index = end + 1;
                } else {
                    out.push('`');
                    index += 1;
                }
            }
            '*' if chars.get(index + 1) == Some(&'*') => {
                if let Some(end) = find_pair(&chars, index + 2, '*', '*') {
                    out.push_str("<strong>");
                    out.push_str(&inline_escaped(
                        &chars[index + 2..end].iter().collect::<String>(),
                    ));
                    out.push_str("</strong>");
                    index = end + 2;
                } else {
                    out.push('*');
                    index += 1;
                }
            }
            '*' => {
                if let Some(end) = find_char(&chars, index + 1, '*') {
                    out.push_str("<em>");
                    out.push_str(&inline_escaped(
                        &chars[index + 1..end].iter().collect::<String>(),
                    ));
                    out.push_str("</em>");
                    index = end + 1;
                } else {
                    out.push('*');
                    index += 1;
                }
            }
            '[' => {
                if let Some((label, url, end)) = parse_link(&chars, index) {
                    match url {
                        Some(url) => {
                            out.push_str("<a href=\"");
                            // Текст уже экранирован: повторное экранирование
                            // превратило бы `&amp;` в `&amp;amp;`.
                            out.push_str(&url);
                            out.push_str("\" rel=\"noreferrer noopener\">");
                            out.push_str(&inline_escaped(&label));
                            out.push_str("</a>");
                        }
                        None => out.push_str(&label),
                    }
                    index = end;
                } else {
                    out.push('[');
                    index += 1;
                }
            }
            ch => {
                out.push(ch);
                index += 1;
            }
        }
    }

    out
}

fn find_char(chars: &[char], start: usize, needle: char) -> Option<usize> {
    (start..chars.len()).find(|index| chars[*index] == needle)
}

fn find_pair(chars: &[char], start: usize, first: char, second: char) -> Option<usize> {
    let mut index = start;
    while index + 1 < chars.len() {
        if chars[index] == first && chars[index + 1] == second {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(String, Option<String>, usize)> {
    let close_label = find_char(chars, start + 1, ']')?;
    if chars.get(close_label + 1) != Some(&'(') {
        return None;
    }
    let close_url = find_char(chars, close_label + 2, ')')?;
    let label: String = chars[start + 1..close_label].iter().collect();
    let url: String = chars[close_label + 2..close_url].iter().collect();
    let lower = url.to_ascii_lowercase();
    let safe = lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with('#')
        || lower.starts_with('/');
    if !safe {
        return Some((format!("{label} ({url})"), None, close_url + 1));
    }
    Some((label, Some(url), close_url + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::site::{ComponentDoc, DocStatus, SiteDoc, SiteStats};

    fn empty_doc() -> SiteDoc {
        SiteDoc {
            project: "demo".to_string(),
            generated_at: "0".to_string(),
            root: "D:/demo".to_string(),
            stats: SiteStats::default(),
            d1_min_factuality: 0.85,
            d1_min_completeness: 3,
            overview_markdown: String::new(),
            overview_html: String::new(),
            overview_status: DocStatus::Unverified,
            overview_verification: None,
            entry_points: Vec::new(),
            components: Vec::new(),
            files: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn embeds_configured_d1_thresholds() {
        let mut doc = empty_doc();
        doc.d1_min_factuality = 0.72;
        doc.d1_min_completeness = 4;
        let html = render_site(&doc);
        assert!(html.contains("\"d1_min_factuality\":0.72"));
        assert!(html.contains("\"d1_min_completeness\":4"));
        assert!(html.contains("Number(DATA.d1_min_factuality || 0.85)"));
    }

    #[test]
    fn escapes_raw_html() {
        let html = markdown_to_html("<script>alert('x')</script>");
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn renders_headings_lists_and_code() {
        let html = markdown_to_html("# Заголовок\n\n- один\n- два\n\n```rust\nfn main() {}\n```\n");
        assert!(html.contains("<h1>Заголовок</h1>"));
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>один</li>"));
        assert!(html.contains("language-rust"));
        assert!(html.contains("fn main() {}"));
    }

    #[test]
    fn renders_inline_markup_and_safe_links() {
        let html = markdown_to_html("**жирный** и `код` и [сайт](https://example.com)");
        assert!(html.contains("<strong>жирный</strong>"));
        assert!(html.contains("<code>код</code>"));
        assert!(html.contains("href=\"https://example.com\""));
        let unsafe_link = markdown_to_html("[click](javascript:alert(1))");
        assert!(!unsafe_link.contains("href="));
        assert!(unsafe_link.contains("click (javascript:alert(1))"));

        let query = markdown_to_html("[поиск](https://example.com/?a=1&b=2)");
        assert!(query.contains("href=\"https://example.com/?a=1&amp;b=2\""));
        assert!(!query.contains("&amp;amp;"));
    }

    #[test]
    fn renders_tables() {
        let html = markdown_to_html("| A | B |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>A</th>"));
        assert!(html.contains("<td>2</td>"));
    }

    #[test]
    fn render_site_is_self_contained_and_escaped() {
        let mut doc = empty_doc();
        doc.project = "demo <script>".to_string();
        doc.components.push(ComponentDoc {
            id: "c1".to_string(),
            name: "Ядро".to_string(),
            description: "Описание".to_string(),
            markdown: String::new(),
            html: String::new(),
            files: vec!["src/main.rs".to_string()],
            children: Vec::new(),
            status: DocStatus::Verified,
            verification: None,
        });
        let html = render_site(&doc);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("const DATA ="));
        assert!(html.contains("demo &lt;script&gt;"));
        assert!(!html.contains("</script><script>"));
        assert!(html.contains("Карта проекта для разработчика"));
        assert!(
            html.contains(".replace(/\\\\/g, '/').replace(/\\/+$/, '')"),
            "нормализация Windows-пути в JS должна быть синтаксически корректной"
        );
    }
}
