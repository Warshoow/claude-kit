<script setup lang="ts">
import { withBase } from 'vitepress'
import DownloadButton from './DownloadButton.vue'

const features = [
  {
    icon: '📦',
    title: 'Plugin library',
    desc: 'Import from the official marketplace or any local folder. Skills, commands, agents, hooks and MCP configs land in one central library you curate over time.',
  },
  {
    icon: '🎯',
    title: 'Bundles',
    desc: 'Group assets that belong together and apply them to any project in one click — relative symlinks under <code>.claude/</code>, no duplication on disk.',
  },
  {
    icon: '🔗',
    title: 'Bundle sharing',
    desc: 'Export any bundle as a compact share code (<code>ck1:…</code>). Content is embedded — hand-edited and AI-harmonized assets travel with the code, no server needed.',
  },
  {
    icon: '✨',
    title: 'AI flows',
    desc: 'Generate assets from a prompt. Or harmonize a whole bundle — assets pulled from different plugins have no shared logic, which creates gaps in your workflow. Harmonization rewrites them together so they form a coherent, functional whole instead of a patchwork. Per-hunk diff review before anything hits disk.',
  },
  {
    icon: '🔄',
    title: 'Plugin updates',
    desc: 'When a plugin gains a new version, review each changed hunk before overwriting. Local edits are never silently replaced.',
  },
  {
    icon: '🛍️',
    title: 'Multiple marketplaces',
    desc: 'Add community or private sources alongside the official catalog — paste a <code>marketplace.json</code> URL and browse everything from one grid.',
  },
]

const steps = [
  {
    n: '01',
    title: 'Import',
    desc: 'Browse the official marketplace or drop in a local folder. Everything lands in <code>~/.claude-assets/library/</code>.',
  },
  {
    n: '02',
    title: 'Bundle',
    desc: 'Group the assets that go together. Name it <code>python-backend</code>, <code>web-frontend</code>, whatever fits.',
  },
  {
    n: '03',
    title: 'Apply',
    desc: 'One click and relative symlinks appear under <code>.claude/</code>. Switch project, apply a different bundle.',
  },
  {
    n: '04',
    title: 'Share',
    desc: 'Export a bundle as a compact code. A teammate pastes it in — assets arrive with all your modifications intact.',
  },
]

const screenshots = [
  { src: '/screenshots/library.png',      caption: 'Browse Library — imported plugins with asset counts' },
  { src: '/screenshots/marketplace.png',  caption: 'Browse Marketplace — official plugin catalog' },
  { src: '/screenshots/bundle-editor.png',caption: 'Bundle editor — pick assets from your library' },
  { src: '/screenshots/asset-editor.png', caption: 'Asset editor — CodeMirror 6 with Markdown highlighting' },
]
</script>

<template>
  <div class="home">

    <!-- ── HERO ───────────────────────────────────── -->
    <section class="hero">
      <div class="hero-glow" aria-hidden="true" />
      <div class="hero-inner">
        <img src="/icon.png" alt="" class="hero-logo" />
        <h1 class="hero-title">claude-kit</h1>
        <p class="hero-headline">
          Bundle your Claude Code assets.<br>Apply them anywhere.
        </p>
        <p class="hero-tagline">
          A desktop app for managing skills, commands, agents, hooks and MCP configs
          as reusable bundles — imported from the marketplace, polished by AI,
          applied to any project via symlinks in <code>.claude/</code>.
        </p>
        <div class="hero-cta">
          <DownloadButton />
        </div>
      </div>
    </section>

    <!-- ── FEATURES ──────────────────────────────── -->
    <section class="sect sect-alt">
      <div class="wrap">
        <h2 class="sect-title">What's inside</h2>
        <div class="feat-grid">
          <div v-for="f in features" :key="f.title" class="feat-card">
            <span class="feat-icon" aria-hidden="true">{{ f.icon }}</span>
            <h3 class="feat-name">{{ f.title }}</h3>
            <p class="feat-desc" v-html="f.desc" />
          </div>
        </div>
      </div>
    </section>

    <!-- ── HOW IT WORKS ──────────────────────────── -->
    <section class="sect">
      <div class="wrap">
        <h2 class="sect-title">How it works</h2>
        <div class="steps">
          <div v-for="s in steps" :key="s.n" class="step">
            <span class="step-n" aria-hidden="true">{{ s.n }}</span>
            <div>
              <h3 class="step-title">{{ s.title }}</h3>
              <p class="step-desc" v-html="s.desc" />
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- ── SCREENSHOTS ───────────────────────────── -->
    <section class="sect sect-alt">
      <div class="wrap">
        <h2 class="sect-title">Screenshots</h2>
        <div class="shots">
          <figure v-for="s in screenshots" :key="s.src" class="shot">
            <img :src="withBase(s.src)" :alt="s.caption" loading="lazy" />
            <figcaption>{{ s.caption }}</figcaption>
          </figure>
        </div>
      </div>
    </section>

    <!-- ── FINAL CTA ──────────────────────────────── -->
    <section class="sect sect-cta">
      <div class="wrap wrap-center">
        <h2 class="sect-title">Ready to start?</h2>
        <p class="cta-sub">Free and open source — MIT License.</p>
        <div class="cta-dl">
          <DownloadButton />
        </div>
        <p class="cta-links">
          <a href="https://github.com/Warshoow/claude-kit" rel="noopener">GitHub</a>
          ·
          <a href="https://github.com/Warshoow/claude-kit/releases" rel="noopener">All releases</a>
          ·
          <a href="https://github.com/Warshoow/claude-kit/blob/master/docs/roadmap.md" rel="noopener">Roadmap</a>
        </p>
      </div>
    </section>

  </div>
</template>

<style scoped>
*, *::before, *::after { box-sizing: border-box; }
p, h1, h2, h3, figure { margin: 0; }

/* ── HERO ───────────────────────────────────────── */
.hero {
  position: relative;
  overflow: hidden;
  display: flex;
  justify-content: center;
  padding: 7rem 1.5rem 6rem;
  background: var(--vp-c-bg);
}
.hero-glow {
  position: absolute;
  inset: 0;
  background: radial-gradient(ellipse 80% 55% at 50% -5%,
    rgba(249, 115, 22, .16) 0%, transparent 65%);
  pointer-events: none;
}
.hero-inner {
  position: relative;
  max-width: 680px;
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 1.1rem;
}
.hero-logo {
  width: 72px;
  height: 72px;
  border-radius: 16px;
}
.hero-title {
  font-size: clamp(2.8rem, 9vw, 4.4rem);
  font-weight: 800;
  letter-spacing: -0.045em;
  line-height: 1;
  background: linear-gradient(115deg, #F97316 25%, #FBBF24 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.hero-headline {
  font-size: clamp(1.1rem, 3vw, 1.35rem);
  font-weight: 600;
  color: var(--vp-c-text-1);
  line-height: 1.4;
}
.hero-tagline {
  font-size: 0.97rem;
  color: var(--vp-c-text-2);
  line-height: 1.7;
  max-width: 520px;
}
.hero-tagline code {
  font-size: 0.875em;
  padding: 0.1em 0.35em;
  border-radius: 4px;
  background: var(--vp-c-bg-alt);
  border: 1px solid var(--vp-c-divider);
}
.hero-cta {
  margin-top: 0.5rem;
}
/* center the download shelf inside the hero */
.hero-cta :deep(.download-shelf) { align-items: center; }
.hero-cta :deep(.cta-row)        { justify-content: center; }
.hero-cta :deep(.hint)           { text-align: center; }

/* ── SECTIONS ────────────────────────────────────── */
.sect      { padding: 5rem 1.5rem; }
.sect-alt  { background: var(--vp-c-bg-alt); }
.wrap      { max-width: 1000px; margin: 0 auto; }
.wrap-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}
.sect-title {
  font-size: 1.6rem;
  font-weight: 700;
  letter-spacing: -0.025em;
  color: var(--vp-c-text-1);
  margin-bottom: 2.5rem;
}

/* ── FEATURES ────────────────────────────────────── */
.feat-grid {
  display: grid;
  gap: 1rem;
  grid-template-columns: 1fr;
}
@media (min-width: 600px) {
  .feat-grid { grid-template-columns: 1fr 1fr; }
}
@media (min-width: 900px) {
  .feat-grid { grid-template-columns: 1fr 1fr 1fr; }
}
.feat-card {
  padding: 1.5rem;
  border-radius: 12px;
  border: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg);
  transition: border-color 200ms ease, box-shadow 200ms ease;
}
.feat-card:hover {
  border-color: rgba(249, 115, 22, .45);
  box-shadow: 0 4px 20px -6px rgba(249, 115, 22, .14);
}
.feat-icon {
  font-size: 1.6rem;
  display: block;
  margin-bottom: 0.75rem;
}
.feat-name {
  font-size: 0.975rem;
  font-weight: 600;
  margin-bottom: 0.45rem;
  color: var(--vp-c-text-1);
}
.feat-desc {
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
  line-height: 1.65;
}
.feat-desc :deep(code) {
  font-size: 0.82em;
  padding: 0.1em 0.3em;
  border-radius: 4px;
  background: var(--vp-c-bg-elv);
  border: 1px solid var(--vp-c-divider);
}

/* ── STEPS ───────────────────────────────────────── */
.steps {
  display: grid;
  gap: 2.5rem;
  grid-template-columns: 1fr;
}
@media (min-width: 640px) {
  .steps { grid-template-columns: 1fr 1fr; }
}
.step {
  display: flex;
  gap: 1.25rem;
  align-items: flex-start;
}
.step-n {
  font-size: 2.2rem;
  font-weight: 800;
  letter-spacing: -0.05em;
  line-height: 1;
  color: #F97316;
  flex-shrink: 0;
  padding-top: 0.05em;
}
.step-title {
  font-size: 1.05rem;
  font-weight: 600;
  margin-bottom: 0.4rem;
  color: var(--vp-c-text-1);
}
.step-desc {
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
  line-height: 1.65;
}
.step-desc :deep(code) {
  font-size: 0.82em;
  padding: 0.1em 0.3em;
  border-radius: 4px;
  background: var(--vp-c-bg-elv);
  border: 1px solid var(--vp-c-divider);
}

/* ── SCREENSHOTS ──────────────────────────────────── */
.shots {
  display: grid;
  gap: 1.5rem;
  grid-template-columns: 1fr;
}
@media (min-width: 640px) {
  .shots { grid-template-columns: 1fr 1fr; }
}
.shot img {
  width: 100%;
  height: auto;
  display: block;
  border-radius: 10px;
  border: 1px solid var(--vp-c-divider);
  box-shadow: 0 4px 24px -8px rgba(0, 0, 0, .16);
  transition: transform 200ms ease, box-shadow 200ms ease;
}
.shot img:hover {
  transform: translateY(-3px);
  box-shadow: 0 10px 32px -8px rgba(249, 115, 22, .2);
}
.shot figcaption {
  margin-top: 0.6rem;
  font-size: 0.8rem;
  color: var(--vp-c-text-2);
  text-align: center;
}

/* ── CTA ──────────────────────────────────────────── */
.sect-cta { background: var(--vp-c-bg); }
.cta-sub {
  font-size: 0.975rem;
  color: var(--vp-c-text-2);
  margin-bottom: 1.5rem;
}
.cta-dl { margin-bottom: 1.25rem; }
.cta-dl :deep(.download-shelf) { align-items: center; }
.cta-dl :deep(.cta-row)        { justify-content: center; }
.cta-dl :deep(.hint)           { text-align: center; }
.cta-links {
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
  line-height: 2;
}
.cta-links a {
  color: var(--vp-c-text-2);
  text-decoration: none;
  transition: color 150ms;
}
.cta-links a:hover { color: #F97316; }
</style>
