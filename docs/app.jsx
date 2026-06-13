/* OpenTokenMonitor — landing page (single-file React)
 * Dev-tool brutalist. Mono, dense, single accent.
 */

const { useState, useEffect } = React;

const REPO = "https://github.com/Hitheshkaranth/OpenTokenMonitor";
const REL = REPO + "/releases/latest/download/";
const DOWNLOADS = [
  { id: "macos-arm",  os: "macOS",   arch: "Apple Silicon", file: "OpenTokenMonitor_aarch64.dmg",      ext: ".dmg",      url: REL + "OpenTokenMonitor_aarch64.dmg"      },
  { id: "windows",    os: "Windows", arch: "x64",           file: "OpenTokenMonitor_x64-setup.msi",    ext: ".msi",      url: REL + "OpenTokenMonitor_x64-setup.msi"    },
  { id: "linux-deb",  os: "Linux",   arch: "Debian / amd64",file: "OpenTokenMonitor_amd64.deb",        ext: ".deb",      url: REL + "OpenTokenMonitor_amd64.deb"        },
  { id: "linux-app",  os: "Linux",   arch: "AppImage",      file: "OpenTokenMonitor_amd64.AppImage",   ext: ".AppImage", url: REL + "OpenTokenMonitor_amd64.AppImage"   },
];

const detectOS = () => {
  if (typeof navigator === "undefined") return "macos-arm";
  const p = (navigator.userAgentData?.platform || navigator.platform || "").toLowerCase();
  const ua = navigator.userAgent.toLowerCase();
  if (p.includes("mac") || ua.includes("mac")) return "macos-arm";
  if (p.includes("win") || ua.includes("win")) return "windows";
  if (p.includes("linux") || ua.includes("linux")) return "linux-deb";
  return "macos-arm";
};

/* ============== NAV ============== */
const Nav = ({ onPickOS }) => (
  <nav className="nav">
    <div className="wrap nav-row">
      <a href="#top" className="nav-brand" aria-label="OpenTokenMonitor home">
        <img className="nav-mark" src="assets/icon.png" alt="" aria-hidden="true"/>
        <span><b>OpenTokenMonitor</b><span className="v">v0.3.5</span></span>
      </a>
      <div className="nav-links">
        <a href="#screens">Product</a>
        <a href="#features">Capabilities</a>
        <a href="#providers">Providers</a>
        <a href="#install">Install</a>
        <a href="#faq">FAQ</a>
      </div>
      <div className="nav-cta">
        <a className="btn btn-ghost" href={REPO} target="_blank" rel="noreferrer">GitHub ↗</a>
        <a className="btn btn-primary" href="#install">Download</a>
      </div>
    </div>
  </nav>
);

/* ============== HERO ============== */
const Hero = ({ os }) => {
  const pick = DOWNLOADS.find(d => d.id === os) || DOWNLOADS[0];
  return (
    <header className="hero" id="top">
      <div className="wrap">
        <div className="hero-grid">
          <div>
            <div className="hero-meta">
              <span className="pip"/>v0.3.5 · open source · MIT
            </div>
            <h1>
              Stop <span className="strike">guessing</span><br/>
              what your AI <em>actually costs.</em>
            </h1>
            <p className="hero-sub">
              OpenTokenMonitor is a local-first desktop app that reads the session
              data your AI CLIs already write — Claude Code, Codex, Antigravity — and
              shows you tokens, costs, and reset timers in one window.
            </p>
            <div className="hero-cta">
              <a className="btn btn-primary" href={pick.url}>
                Download for {pick.os} <span className="arrow">→</span>
              </a>
              <a className="btn" href="#install">All platforms</a>
              <a className="btn btn-ghost" href={REPO} target="_blank" rel="noreferrer">View source ↗</a>
            </div>
            <div className="hero-fineprint">
              ~12 MB · No account · No telemetry · Reads <code style={{padding:"1px 5px",fontSize:11,color:"var(--ink-1)",background:"var(--bg-1)",borderRadius:3,border:"1px solid var(--line)"}}>~/.claude</code> <code style={{padding:"1px 5px",fontSize:11,color:"var(--ink-1)",background:"var(--bg-1)",borderRadius:3,border:"1px solid var(--line)"}}>~/.codex</code> <code style={{padding:"1px 5px",fontSize:11,color:"var(--ink-1)",background:"var(--bg-1)",borderRadius:3,border:"1px solid var(--line)"}}>~/.gemini/antigravity-cli</code>
            </div>
          </div>
          <div className="hero-shot">
            <span className="corner tr1"/><span className="corner tr2"/>
            <span className="corner bl1"/><span className="corner bl2"/>
            <span className="corner br1"/><span className="corner br2"/>
            <img src="assets/overview-0.3.1.png" alt="OpenTokenMonitor dashboard with per-provider usage rings, session and weekly windows, and 30-day cost trends."/>
          </div>
        </div>
      </div>
      <div className="wrap">
        <div className="hero-meter">
          <div className="cell"><div className="k">Providers</div><div className="v">Claude · Codex · Antigravity</div></div>
          <div className="cell"><div className="k">Runs on</div><div className="v">macOS · Windows · Linux</div></div>
          <div className="cell"><div className="k">Data</div><div className="v"><span className="acc">●</span> 100% local</div></div>
          <div className="cell"><div className="k">License</div><div className="v">MIT</div></div>
        </div>
      </div>
    </header>
  );
};

/* ============== PROBLEM ============== */
const Problem = () => (
  <section className="problem">
    <div className="wrap">
      <p className="problem-q">
        You ship code with three AI tools. At the end of the month, you have one bill — and zero idea who used what.
      </p>
      <h2 className="problem-a">
        Your CLIs already log everything. <em>Read it.</em>
      </h2>
      <div className="problem-by">A small, opinionated tool by Hithesh Karanth</div>
    </div>
  </section>
);

/* ============== SCREENSHOTS ============== */
const SHOTS = [
  {
    id: "overview",
    src: "assets/overview-0.3.1.png",
    num: "01",
    kind: "Dashboard",
    title: "Every provider on one screen.",
    desc: "Three usage rings. Three trendlines. Session and weekly windows side-by-side. The full state of your AI consumption, no scrolling.",
    feats: [
      ["Layout", "Three-up provider stack"],
      ["Windows", "5H · SES · 7D · WK"],
      ["Costs", "30-day total per provider"],
      ["Models", "Per-row model attribution"],
    ],
  },
  {
    id: "widget",
    src: "assets/widget-0.3.1.png",
    num: "02",
    kind: "Widget",
    title: "Pin it. Forget it.",
    desc: "Always-on-top compact mode. Park it in a corner of your monitor and glance over while you code. Same data, smaller footprint.",
    feats: [
      ["Layout", "Three-up provider columns"],
      ["Modes", "SES · WK · DAY toggles"],
      ["Resets", "Live countdown per window"],
      ["Footprint", "376 × 283"],
    ],
  },
];

const Screens = () => (
  <section id="screens">
    <div className="wrap">
      <div className="sec-head">
        <div className="sec-tag"><span className="num">02</span>The product</div>
        <div>
          <h2 className="sec-h2">Real screenshots. <em>No mockups.</em></h2>
          <p className="sec-lede">
            The dashboard ships in two surfaces — a full window for analysis and a
            compact widget for ambient awareness. Both pull from the same local index.
          </p>
        </div>
      </div>
    </div>
    <div className="shots">
      {SHOTS.map(s => (
        <div className="shot" key={s.id}>
          <div className="shot-head">
            <div className="shot-num"><b>{s.num}</b>{s.kind}</div>
            <div className="shot-kind">v0.3.5</div>
          </div>
          <div className="shot-img">
            <img src={s.src} alt={`OpenTokenMonitor ${s.kind} screenshot`} loading="lazy"/>
          </div>
          <div className="shot-meta">
            <h3>{s.title}</h3>
            <p>{s.desc}</p>
            <ul className="shot-feats">
              {s.feats.map(([k, v]) => (
                <li key={k}><span className="k">{k}</span><span>{v}</span></li>
              ))}
            </ul>
          </div>
        </div>
      ))}
    </div>
  </section>
);

/* ============== FEATURES ============== */
const FEATS = [
  { k: "USAGE",   t: "Token totals",         d: "Per-session, per-model, per-day. Aggregated from local CLI session files."},
  { k: "COSTS",   t: "Live cost trends",     d: "30-day windows with model-aware pricing. See where your spend is actually going."},
  { k: "WINDOWS", t: "Reset timers",         d: "Session, weekly, and daily limit countdowns surfaced for each provider."},
  { k: "MODELS",  t: "Per-model attribution",d: "Which model burned which tokens. Opus vs Sonnet, GPT-5 vs mini — split out."},
  { k: "WIDGET",  t: "Always-on-top",        d: "Compact widget mode. Pin to a corner and keep an eye on usage as you work."},
  { k: "LOCAL",   t: "No cloud required",    d: "Everything reads, parses, and stores on your machine. No accounts, no upload."},
  { k: "OPEN",    t: "MIT licensed",         d: "Source on GitHub. Read it, audit it, fork it. No black boxes."},
  { k: "FAST",    t: "Native binary",        d: "Tauri-built. ~12 MB install. Cold start under a second on modern hardware."},
  { k: "PROJECTS",t: "Per-project view",     d: "Group sessions by working directory. See which repo is eating your budget."},
];

const Features = () => (
  <section id="features">
    <div className="wrap">
      <div className="sec-head">
        <div className="sec-tag"><span className="num">03</span>Capabilities</div>
        <div>
          <h2 className="sec-h2">Nine things it does. <em>Zero it shouldn't.</em></h2>
          <p className="sec-lede">
            Built around a single discipline: read the data your tools already produce
            and present it without ceremony.
          </p>
        </div>
      </div>
    </div>
    <div className="feats">
      {FEATS.map(f => (
        <div className="feat" key={f.k}>
          <div className="feat-k">{f.k}</div>
          <h3>{f.t}</h3>
          <p>{f.d}</p>
        </div>
      ))}
    </div>
  </section>
);

/* ============== PROVIDERS ============== */
const PROVS = [
  { id: "claude", name: "Claude",  cli: "Claude Code",      path: "~/.claude",  status: "Auto-detect" },
  { id: "codex",  name: "Codex",   cli: "OpenAI Codex CLI", path: "~/.codex",   status: "Auto-detect" },
  { id: "antigravity", name: "Antigravity", cli: "Antigravity CLI/IDE", path: "~/.gemini/antigravity-cli", status: "Auto-detect" },
];

const Providers = () => (
  <section id="providers">
    <div className="wrap">
      <div className="sec-head">
        <div className="sec-tag"><span className="num">04</span>Providers</div>
        <div>
          <h2 className="sec-h2">Three CLIs. <em>One window.</em></h2>
          <p className="sec-lede">
            OpenTokenMonitor reads what these tools already write to disk. Add API
            keys only if you want richer provider-side data.
          </p>
        </div>
      </div>
      <div className="prov-table">
        {PROVS.map(p => (
          <div className="prov-row" key={p.id}>
            <div className="prov-name">
              {p.name}<span className="badge">{p.cli}</span>
            </div>
            <div className="prov-desc">
              Reads local <code>{p.path}</code> session artifacts and surfaces
              tokens, costs, models, and reset windows in a dedicated provider page.
            </div>
            <div className="prov-status">
              <div className="row"><span className="ok">●</span><span>{p.status}</span></div>
              <div style={{marginTop:6,color:"var(--ink-3)"}}>{p.path}</div>
            </div>
          </div>
        ))}
      </div>
      <p style={{marginTop: 24, fontSize: 11, color: "var(--ink-3)", letterSpacing: "0.04em"}}>
        OpenTokenMonitor is independent and not affiliated with Anthropic, OpenAI, or Google.
        Provider names are used for compatibility reference only.
      </p>
    </div>
  </section>
);

/* ============== INSTALL ============== */
const Install = ({ os }) => (
  <section id="install">
    <div className="wrap">
      <div className="sec-head">
        <div className="sec-tag"><span className="num">05</span>Install</div>
        <div>
          <h2 className="sec-h2">Pick a binary. <em>Or build from source.</em></h2>
          <p className="sec-lede">
            Native installers for the three major desktop OSes. Or one command if
            you'd rather build it yourself.
          </p>
        </div>
      </div>
    </div>
    <div className="wrap sec-body">
      <div className="install-grid">
        <div className="install-card">
          <h3>Releases<span className="meta">v0.3.5</span></h3>
          <p>Direct downloads from GitHub Releases. Auto-updating is on the roadmap.</p>
          <ul className="install-list">
            {DOWNLOADS.map(d => (
              <li key={d.id}>
                <a href={d.url}>
                  <span className="label">
                    <b>{d.os}</b>
                    <span className="ext">{d.arch} · {d.ext}</span>
                    {d.id === os && <span className="recommended">Recommended</span>}
                  </span>
                  <span className="arrow">↓</span>
                </a>
              </li>
            ))}
          </ul>
        </div>
        <div className="install-card">
          <h3>From source<span className="meta">git · npm · rust</span></h3>
          <p>Tauri app. Requires Node 18+ and a recent Rust toolchain.</p>
          <pre className="code-block">
<span className="comment"># clone</span>{"\n"}
<span className="prompt">$</span> git clone {REPO}.git{"\n"}
<span className="prompt">$</span> cd OpenTokenMonitor{"\n"}
{"\n"}
<span className="comment"># install + build</span>{"\n"}
<span className="prompt">$</span> npm install{"\n"}
<span className="prompt">$</span> npm run tauri build{"\n"}
          </pre>
          <a className="btn" href={REPO} target="_blank" rel="noreferrer" style={{marginTop:14}}>
            Read the README ↗
          </a>
        </div>
      </div>
    </div>
  </section>
);

/* ============== STEPS ============== */
const Steps = () => (
  <section>
    <div className="wrap">
      <div className="sec-head">
        <div className="sec-tag"><span className="num">06</span>How it works</div>
        <div>
          <h2 className="sec-h2">Three steps. <em>One minute.</em></h2>
          <p className="sec-lede">
            No accounts. No services to deploy. No config files unless you want them.
          </p>
        </div>
      </div>
    </div>
    <div className="steps">
      <div className="step-cell">
        <span className="step-num">i</span>
        <h3>Install</h3>
        <p>Run the installer for your OS, or <code style={{fontSize:12,color:"var(--ink-1)",background:"var(--bg-1)",padding:"1px 5px",borderRadius:3,border:"1px solid var(--line)"}}>npm run tauri build</code> from source.</p>
      </div>
      <div className="step-cell">
        <span className="step-num">ii</span>
        <h3>Open</h3>
        <p>The app auto-detects Claude, Codex, and Antigravity CLI/IDE artifacts on first launch. Nothing else to configure.</p>
      </div>
      <div className="step-cell">
        <span className="step-num">iii</span>
        <h3>Watch</h3>
        <p>Tokens and costs update in real time. Pin the widget; tab through providers; export when you need to.</p>
      </div>
    </div>
  </section>
);

/* ============== PRIVACY ============== */
const Privacy = () => (
  <section>
    <div className="wrap sec-body">
      <div className="manifest">
        <div>
          <div className="sec-tag" style={{marginBottom:20}}><span className="num">07</span>Privacy</div>
          <h2>Your data <em>never has to leave</em> your machine.</h2>
          <p>
            OpenTokenMonitor reads what your CLI tools already write locally and
            keeps the parsed index on your device. No telemetry. No analytics ping.
            No cloud requirement for monitoring.
          </p>
          <p>
            If you want richer provider-side data — like API-only usage that doesn't
            land in CLI logs — you can opt-in to API keys. They stay in OS keychain.
          </p>
        </div>
        <ul>
          <li>Stays on your device by default</li>
          <li>No telemetry or analytics</li>
          <li>Local-only parsed index</li>
          <li>Open source, MIT licensed</li>
          <li>API credentials are opt-in only</li>
          <li>Auditable from end to end</li>
        </ul>
      </div>
    </div>
  </section>
);

/* ============== FAQ ============== */
const FAQS = [
  {
    q: "Do I need API keys to use it?",
    a: <>No. The default mode reads the session files your CLIs already write to <code>~/.claude</code>, <code>~/.codex</code>, and <code>~/.gemini/antigravity-cli</code>. API keys are only needed if you want provider-side data that isn't in local logs.</>,
  },
  {
    q: "Is my data sent anywhere?",
    a: <>No. Parsing and storage happen entirely on your machine. There is no backend, no analytics endpoint, and no auto-update telemetry in this build.</>,
  },
  {
    q: "Which platforms are supported?",
    a: <>macOS (Apple Silicon), Windows (x64), and Linux (Debian and AppImage). Intel Mac builds aren't currently shipped — build from source if you need one.</>,
  },
  {
    q: "Is it really free?",
    a: <>Yes. MIT licensed, source on GitHub. There is no paid tier, no "pro" version, and no plan to add one.</>,
  },
  {
    q: "What about other AI tools — Cursor, Aider, etc.?",
    a: <>v0.3.5 supports Claude Code, OpenAI Codex CLI, and Antigravity CLI/IDE. Other tools are on the roadmap; the parser is provider-pluggable, so PRs are welcome.</>,
  },
  {
    q: "Why a desktop app and not a CLI?",
    a: <>Because reading numbers in a grid is faster than running <code>jq</code> over five log directories. It's also a native app — Tauri-built, ~12 MB, no Electron.</>,
  },
];

const Faq = () => (
  <section>
    <div className="wrap">
      <div className="sec-head">
        <div className="sec-tag"><span className="num">08</span>FAQ</div>
        <div>
          <h2 className="sec-h2">Questions, <em>answered.</em></h2>
          <p className="sec-lede">If yours isn't here, open an issue on GitHub.</p>
        </div>
      </div>
    </div>
    <div className="wrap sec-body">
      <div className="faq" id="faq">
        {FAQS.map((f, i) => (
          <details key={i}>
            <summary>
              <span className="num">{String(i+1).padStart(2,"0")}</span>
              <span>{f.q}</span>
              <span className="toggle">+</span>
            </summary>
            <div className="faq-body">{f.a}</div>
          </details>
        ))}
      </div>
    </div>
  </section>
);

/* ============== FOOTER ============== */
const Footer = () => (
  <footer className="footer">
    <div className="wrap">
      <div className="footer-grid">
        <div className="footer-brand">
          <div className="nav-brand">
            <span className="nav-mark" aria-hidden="true">
              <span className="on"/><span className="on"/><span className="on"/><span className="on"/>
              <span className="on"/><span className="on"/><span className="on"/><span className="on"/>
              <span/><span/><span/><span/>
              <span/><span/><span/><span/>
            </span>
            <span><b>OpenTokenMonitor</b></span>
          </div>
          <p>
            A local-first token + cost monitor for Claude Code, Codex, and Antigravity.
            Built by Hithesh Karanth. MIT licensed.
          </p>
        </div>
        <div className="footer-col">
          <h4>Product</h4>
          <ul>
            <li><a href="#screens">Screenshots</a></li>
            <li><a href="#features">Capabilities</a></li>
            <li><a href="#providers">Providers</a></li>
            <li><a href="#install">Download</a></li>
          </ul>
        </div>
        <div className="footer-col">
          <h4>Source</h4>
          <ul>
            <li><a href={REPO} target="_blank" rel="noreferrer">GitHub ↗</a></li>
            <li><a href={REPO + "/releases"} target="_blank" rel="noreferrer">Releases ↗</a></li>
            <li><a href={REPO + "/issues"} target="_blank" rel="noreferrer">Issues ↗</a></li>
            <li><a href={REPO + "/blob/main/LICENSE"} target="_blank" rel="noreferrer">License ↗</a></li>
          </ul>
        </div>
        <div className="footer-col">
          <h4>About</h4>
          <ul>
            <li><a href="#faq">FAQ</a></li>
            <li><a href="#">Privacy</a></li>
            <li><a href="https://github.com/Hitheshkaranth" target="_blank" rel="noreferrer">@Hitheshkaranth ↗</a></li>
          </ul>
        </div>
      </div>
      <div className="footer-bottom">
        <div><span className="pip"/>v0.3.5 · MIT · © 2026 Hithesh Karanth</div>
        <div>Not affiliated with Anthropic, OpenAI, or Google.</div>
      </div>
    </div>
  </footer>
);

/* ============== APP ============== */
const App = () => {
  const [os, setOs] = useState("macos-arm");
  useEffect(() => { setOs(detectOS()); }, []);

  return (
    <>
      <Nav/>
      <Hero os={os}/>
      <Problem/>
      <Screens/>
      <Features/>
      <Providers/>
      <Install os={os}/>
      <Steps/>
      <Privacy/>
      <Faq/>
      <Footer/>
    </>
  );
};

ReactDOM.createRoot(document.getElementById("root")).render(<App/>);
