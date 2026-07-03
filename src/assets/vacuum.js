// Launch animation: a localized "black hole" that pulls a handful of pixels
// inward around the ring, then spits a burst outward as the target opens.
// The effect is confined to a small radius around the ring centre — the rest
// of the fullscreen overlay stays fully transparent (and click-through).
(function () {
  const canvas = document.getElementById("vacuum");
  const ctx = canvas.getContext("2d");

  function fit() {
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.floor(window.innerWidth * dpr);
    canvas.height = Math.floor(window.innerHeight * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  function smooth(p) { return p * p * (3 - 2 * p); }

  // Inward pull + accelerating spiral over ~2.6s.
  // `center` is {x, y} in CSS px within the overlay window (screen-relative on
  // Windows; window-centre on Linux/Wayland — the caller decides).
  function suck(center, ringPx) {
    return new Promise((resolve) => {
      canvas.classList.remove("hidden");
      fit();
      const cx = center.x;
      const cy = center.y;
      const R = Math.max(150, ringPx * 9); // confined pull radius
      const DURATION = 2600;

      const parts = [];
      for (let i = 0; i < 130; i++) {
        const r = R * (0.35 + Math.random() * 0.75);
        parts.push({
          a: Math.random() * Math.PI * 2,
          r0: r,
          spin: 0.6 + Math.random() * 1.5,
          size: 0.6 + Math.random() * 1.9,
          white: Math.random() < 0.5,
          delay: Math.random() * 0.25,
        });
      }
      const streaks = [];
      for (let i = 0; i < 10; i++) {
        streaks.push({ a: Math.random() * Math.PI * 2, len: 0.2 + Math.random() * 0.3 });
      }

      let start = null;
      function frame(t) {
        if (start === null) start = t;
        let p = (t - start) / DURATION;
        if (p > 1) p = 1;
        const e = smooth(p);
        ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);

        // Confined dark gravity well.
        const well = ctx.createRadialGradient(cx, cy, 0, cx, cy, R * 1.15);
        well.addColorStop(0, `rgba(2,3,8,${0.78 * e})`);
        well.addColorStop(0.5, `rgba(3,5,14,${0.5 * e})`);
        well.addColorStop(1, "rgba(0,0,0,0)");
        ctx.fillStyle = well;
        ctx.beginPath();
        ctx.arc(cx, cy, R * 1.15, 0, Math.PI * 2);
        ctx.fill();

        // Thin white lines flowing inward.
        ctx.lineCap = "round";
        ctx.lineWidth = 1.1;
        for (const s of streaks) {
          const rr = R * (1 - e * 0.92);
          const a = s.a + e * 3.0;
          const x1 = cx + Math.cos(a) * rr;
          const y1 = cy + Math.sin(a) * rr;
          const x2 = cx + Math.cos(a) * rr * (1 - s.len);
          const y2 = cy + Math.sin(a) * rr * (1 - s.len);
          ctx.strokeStyle = `rgba(220,235,255,${0.12 + 0.5 * e})`;
          ctx.beginPath();
          ctx.moveTo(x1, y1);
          ctx.lineTo(x2, y2);
          ctx.stroke();
        }

        // Particles spiral inward.
        for (const pt of parts) {
          const lp = Math.max(0, (e - pt.delay) / (1 - pt.delay));
          const clp = Math.min(1, lp);
          const r = pt.r0 * (1 - clp);
          const a = pt.a + clp * pt.spin * 6.0;
          const x = cx + Math.cos(a) * r;
          const y = cy + Math.sin(a) * r;
          const alpha = Math.min(1, 0.2 + clp) * (1 - clp * 0.15);
          ctx.fillStyle = pt.white
            ? `rgba(240,246,255,${alpha})`
            : `rgba(95,155,255,${alpha * 0.9})`;
          ctx.beginPath();
          ctx.arc(x, y, pt.size * (1 - clp * 0.4), 0, Math.PI * 2);
          ctx.fill();
        }

        // Brightening core.
        const coreR = Math.max(6, ringPx * 1.2) * (0.6 + e);
        const core = ctx.createRadialGradient(cx, cy, 0, cx, cy, coreR);
        core.addColorStop(0, `rgba(130,185,255,${0.55 * e})`);
        core.addColorStop(1, "rgba(130,185,255,0)");
        ctx.fillStyle = core;
        ctx.beginPath();
        ctx.arc(cx, cy, coreR, 0, Math.PI * 2);
        ctx.fill();

        if (p < 1) requestAnimationFrame(frame);
        else resolve();
      }
      requestAnimationFrame(frame);
    });
  }

  // Outward burst as the app/folder is "spat out" (~0.5s).
  function spit(center) {
    return new Promise((resolve) => {
      const cx = center.x;
      const cy = center.y;
      const DURATION = 520;
      const parts = [];
      for (let i = 0; i < 90; i++) {
        parts.push({ a: Math.random() * Math.PI * 2, v: 3 + Math.random() * 7, size: 0.8 + Math.random() * 2 });
      }
      let start = null;
      function frame(t) {
        if (start === null) start = t;
        let p = (t - start) / DURATION;
        if (p > 1) p = 1;
        ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);

        // Expanding flash ring.
        ctx.strokeStyle = `rgba(185,218,255,${(1 - p) * 0.8})`;
        ctx.lineWidth = 2.5 * (1 - p) + 0.5;
        ctx.beginPath();
        ctx.arc(cx, cy, p * 220, 0, Math.PI * 2);
        ctx.stroke();

        // Central flash.
        const flash = ctx.createRadialGradient(cx, cy, 0, cx, cy, 46);
        flash.addColorStop(0, `rgba(255,255,255,${(1 - p) * 0.9})`);
        flash.addColorStop(1, "rgba(165,205,255,0)");
        ctx.fillStyle = flash;
        ctx.beginPath();
        ctx.arc(cx, cy, 46, 0, Math.PI * 2);
        ctx.fill();

        // Outflying particles.
        for (const pt of parts) {
          const d = p * pt.v * 22;
          const x = cx + Math.cos(pt.a) * d;
          const y = cy + Math.sin(pt.a) * d;
          ctx.fillStyle = `rgba(232,242,255,${1 - p})`;
          ctx.beginPath();
          ctx.arc(x, y, pt.size * (1 - p), 0, Math.PI * 2);
          ctx.fill();
        }

        if (p < 1) {
          requestAnimationFrame(frame);
        } else {
          ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
          canvas.classList.add("hidden");
          resolve();
        }
      }
      requestAnimationFrame(frame);
    });
  }

  window.Vacuum = { suck, spit };
})();
