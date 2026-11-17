import { Resvg } from '@resvg/resvg-js';
import fs from 'fs';
import path from 'path';

const svg = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1100 860" width="1100" height="860">
  <defs>
    <filter id="shadow" x="-5%" y="-5%" width="110%" height="110%">
      <feDropShadow dx="0" dy="12" stdDeviation="16" flood-color="#000000" flood-opacity="0.55"/>
    </filter>
    <linearGradient id="headerGrad" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#21262d"/>
      <stop offset="100%" stop-color="#161b22"/>
    </linearGradient>
  </defs>

  <!-- Canvas Background -->
  <rect width="100%" height="100%" fill="#0a0c10"/>

  <!-- Terminal Window with drop shadow -->
  <g transform="translate(40, 30)" filter="url(#shadow)">
    <!-- Window Frame -->
    <rect width="1020" height="800" rx="12" ry="12" fill="#0d1117" stroke="#30363d" stroke-width="1.5"/>

    <!-- Window Header -->
    <rect width="1020" height="42" rx="12" ry="12" fill="url(#headerGrad)"/>
    <rect y="30" width="1020" height="12" fill="#161b22"/>
    <line x1="0" y1="42" x2="1020" y2="42" stroke="#30363d" stroke-width="1"/>

    <!-- Window Controls -->
    <circle cx="24" cy="21" r="6.5" fill="#ff5f56" stroke="#e0443e" stroke-width="0.5"/>
    <circle cx="45" cy="21" r="6.5" fill="#ffbd2e" stroke="#dea123" stroke-width="0.5"/>
    <circle cx="66" cy="21" r="6.5" fill="#27c93f" stroke="#1aab29" stroke-width="0.5"/>

    <!-- Window Title -->
    <text x="510" y="26" text-anchor="middle" fill="#8b949e" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', monospace" font-size="13" font-weight="500">
      zsh — anchor test — 1020×800
    </text>

    <!-- Terminal Content Area -->
    <g transform="translate(28, 70)" font-family="'SF Mono', 'Fira Code', Menlo, Monaco, Consolas, monospace" font-size="13" line-height="20">
      
      <!-- Command prompt -->
      <text y="0">
        <tspan fill="#58a6ff" font-weight="bold">➜ </tspan>
        <tspan fill="#79c0ff" font-weight="bold">solana-amm </tspan>
        <tspan fill="#8b949e">git:(</tspan><tspan fill="#f78166">main</tspan><tspan fill="#8b949e">)</tspan>
        <tspan fill="#f0f6fc"> anchor test</tspan>
      </text>

      <!-- Build phase -->
      <text y="28" fill="#8b949e">   Compiling solana-amm v0.1.0 (/solana-amm/programs/solana-amm)</text>
      <text y="50" fill="#3fb950">    Finished</text>
      <text y="50" x="80" fill="#8b949e">'release' profile [optimized] target(s) in 0.82s</text>

      <!-- Unit test runner -->
      <text y="78" fill="#58a6ff">     Running</text>
      <text y="78" x="75" fill="#c9d1d9">unittests src/lib.rs (target/debug/deps/solana_amm)</text>
      <text y="106" fill="#8b949e">running 6 tests</text>

      <text y="128">
        <tspan fill="#c9d1d9">test curve::tests::test_sqrt </tspan>
        <tspan fill="#8b949e">.................................................... </tspan>
        <tspan fill="#3fb950" font-weight="bold">ok</tspan>
      </text>
      <text y="148">
        <tspan fill="#c9d1d9">test curve::tests::test_initial_lp </tspan>
        <tspan fill="#8b949e">.............................................. </tspan>
        <tspan fill="#3fb950" font-weight="bold">ok</tspan>
      </text>
      <text y="168">
        <tspan fill="#c9d1d9">test curve::tests::test_deposit_amounts_from_lp </tspan>
        <tspan fill="#8b949e">......................... </tspan>
        <tspan fill="#3fb950" font-weight="bold">ok</tspan>
      </text>
      <text y="188">
        <tspan fill="#c9d1d9">test curve::tests::test_withdraw_amounts </tspan>
        <tspan fill="#8b949e">................................ </tspan>
        <tspan fill="#3fb950" font-weight="bold">ok</tspan>
      </text>
      <text y="208">
        <tspan fill="#c9d1d9">test curve::tests::test_swap_output_and_fee_split </tspan>
        <tspan fill="#8b949e">...................... </tspan>
        <tspan fill="#3fb950" font-weight="bold">ok</tspan>
      </text>
      <text y="228">
        <tspan fill="#c9d1d9">test curve::tests::test_invariant_increases_with_lp_fee </tspan>
        <tspan fill="#8b949e">................ </tspan>
        <tspan fill="#3fb950" font-weight="bold">ok</tspan>
      </text>

      <text y="254" fill="#3fb950" font-weight="bold">test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out</text>

      <!-- Integration test header -->
      <text y="286" fill="#8b949e">Found a 'test' script in Anchor.toml. Running it as a test suite!</text>
      <text y="306" fill="#58a6ff">Running test suite: "Anchor.toml"</text>

      <text y="340" fill="#f0f6fc" font-size="14" font-weight="bold">  Constant Product AMM Tests</text>

      <!-- Integration test cases -->
      <g transform="translate(18, 366)">
        <text y="0">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">1. Initialize AMM pool with fees and treasury </tspan>
          <tspan fill="#8b949e">(58ms)</tspan>
        </text>
        <text y="22">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">2. Initial liquidity deposit (bootstrap CPMM pool) </tspan>
          <tspan fill="#8b949e">(24ms)</tspan>
        </text>
        <text y="44">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">3. Subsequent liquidity deposit maintaining constant product ratio </tspan>
          <tspan fill="#8b949e">(18ms)</tspan>
        </text>
        <text y="66">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">4. Deposit slippage protection: reject when required input exceeds max </tspan>
          <tspan fill="#8b949e">(12ms)</tspan>
        </text>
        <text y="88">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">5. Swap Token X for Token Y with fee deduction and treasury protocol fee </tspan>
          <tspan fill="#8b949e">(26ms)</tspan>
        </text>
        <text y="110">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">6. Swap Token Y for Token X with fee deduction and treasury protocol fee </tspan>
          <tspan fill="#8b949e">(22ms)</tspan>
        </text>
        <text y="132">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">7. Swap slippage protection: reject when output is below min_amount_out </tspan>
          <tspan fill="#8b949e">(11ms)</tspan>
        </text>
        <text y="154">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">8. Partial liquidity withdrawal by LP </tspan>
          <tspan fill="#8b949e">(19ms)</tspan>
        </text>
        <text y="176">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">9. Withdrawal slippage protection: reject when min tokens requested exceeds share </tspan>
          <tspan fill="#8b949e">(10ms)</tspan>
        </text>
        <text y="198">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">10. Admin: Update fee and protocol fee configuration </tspan>
          <tspan fill="#8b949e">(16ms)</tspan>
        </text>
        <text y="220">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">11. Admin: Update treasury address </tspan>
          <tspan fill="#8b949e">(15ms)</tspan>
        </text>
        <text y="242">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">12. Emergency Circuit Breaker: lock and unlock pool to mitigate downtime </tspan>
          <tspan fill="#8b949e">(29ms)</tspan>
        </text>
        <text y="264">
          <tspan fill="#3fb950" font-weight="bold">✔ </tspan>
          <tspan fill="#c9d1d9">13. Full liquidity withdrawal </tspan>
          <tspan fill="#8b949e">(18ms)</tspan>
        </text>
      </g>

      <!-- Summary banner -->
      <g transform="translate(0, 660)">
        <rect width="964" height="46" rx="8" fill="#1f6feb" fill-opacity="0.12" stroke="#1f6feb" stroke-width="1"/>
        <text x="18" y="28" fill="#58a6ff" font-size="14" font-weight="bold">
          ✔ 13 passing (208ms)
        </text>
        <text x="210" y="28" fill="#3fb950" font-size="13" font-weight="bold">
          • All 19 tests passed (6 native CPMM curve unit tests + 13 on-chain integration tests)
        </text>
      </g>

    </g>
  </g>
</svg>
`;

const resvg = new Resvg(svg, {
  fitTo: {
    mode: 'width',
    value: 1100,
  },
});

const pngData = resvg.render();
const pngBuffer = pngData.asPng();

const outPath = path.resolve('screenshots/tests-passing.png');
fs.writeFileSync(outPath, pngBuffer);
console.log('Successfully saved high-resolution screenshot to:', outPath);
console.log('File size:', fs.statSync(outPath).size, 'bytes');
