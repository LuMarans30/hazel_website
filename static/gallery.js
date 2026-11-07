(() => {
  const dialog = document.getElementById("lightbox");
  const thumbs = [...document.querySelectorAll(".thumb")];

  if (dialog && typeof dialog.showModal === "function" && thumbs.length) {
    const img = dialog.querySelector("#lightbox-img");
    const title = dialog.querySelector("#lightbox-title");
    const details = dialog.querySelector("#lightbox-details");
    const count = dialog.querySelector("#lightbox-count");
    const single = thumbs.length === 1;
    const wrap = (i) => (i + thumbs.length) % thumbs.length;
    const idOf = (i) => thumbs[i].closest("li").id;

    let index = 0;

    dialog.classList.toggle("single", single);
    img.addEventListener("load", () => img.classList.add("ready"));

    const show = (i) => {
      reset();
      index = wrap(i);
      const t = thumbs[index];

      if (img.getAttribute("src") !== t.dataset.full) {
        img.classList.remove("ready");
        img.src = t.dataset.full;
      }
      img.alt = t.dataset.alt;
      title.textContent = t.dataset.title;
      details.textContent = t.dataset.details;
      count.textContent = single ? "" : `${index + 1} / ${thumbs.length}`;

      history.replaceState(null, "", `#${idOf(index)}`);

      if (!single) {
        for (const n of [index + 1, index - 1]) new Image().src = thumbs[wrap(n)].dataset.full;
      }
    };

    const open = (i) => {
      show(i);
      if (!dialog.open) dialog.showModal();
    };

    const figure = img.closest("figure");
    const MAX_SCALE = 5;
    const DOUBLE_TAP_SCALE = 2.5;
    const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

    let scale = 1;
    let tx = 0;
    let ty = 0;

    const apply = (smooth = false) => {
      const mx = (img.offsetWidth * (scale - 1)) / 2;
      const my = (img.offsetHeight * (scale - 1)) / 2;
      tx = clamp(tx, -mx, mx);
      ty = clamp(ty, -my, my);
      img.classList.toggle("smooth", smooth);
      img.style.transform = scale === 1 ? "" : `translate(${tx}px, ${ty}px) scale(${scale})`;
      dialog.classList.toggle("zoomed", scale > 1);
    };

    const reset = (smooth = false) => {
      scale = 1;
      tx = ty = 0;
      apply(smooth);
    };

    const home = () => {
      const r = figure.getBoundingClientRect();
      return [r.left + r.width / 2, r.top + img.offsetHeight / 2];
    };

    const zoomAt = (px, py, next, smooth = false) => {
      next = clamp(next, 1, MAX_SCALE);
      const [cx, cy] = home();
      const k = next / scale;
      tx = px - cx - (px - cx - tx) * k;
      ty = py - cy - (py - cy - ty) * k;
      scale = next;
      apply(smooth);
    };

    const pointers = new Map();
    let down = null;
    let pan = null;
    let pinch = null;
    let multi = false;
    let lastTap = null;
    let suppressClick = false;

    const pair = () => {
      const [a, b] = [...pointers.values()];
      return { dist: Math.hypot(a.x - b.x, a.y - b.y) || 1, x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    };

    dialog.addEventListener("pointerdown", (e) => {
      if (e.target.closest("button")) return;
      if (e.pointerType === "mouse" && e.button !== 0) return;
      e.target.setPointerCapture(e.pointerId);
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });

      if (pointers.size === 1) {
        multi = false;
        down = { x: e.clientX, y: e.clientY };
        pan = { x: e.clientX, y: e.clientY, tx, ty };
      } else if (pointers.size === 2) {
        multi = true;
        pan = null;
        pinch = { ...pair(), scale, tx, ty, home: home() };
      }
    });

    dialog.addEventListener("pointermove", (e) => {
      const p = pointers.get(e.pointerId);
      if (!p) return;
      p.x = e.clientX;
      p.y = e.clientY;

      if (pinch && pointers.size >= 2) {
        const now = pair();
        const next = clamp((pinch.scale * now.dist) / pinch.dist, 1, MAX_SCALE);
        const k = next / pinch.scale;
        const [cx, cy] = pinch.home;
        tx = now.x - cx - (pinch.x - cx - pinch.tx) * k;
        ty = now.y - cy - (pinch.y - cy - pinch.ty) * k;
        scale = next;
        apply();
      } else if (pan && scale > 1) {
        tx = pan.tx + e.clientX - pan.x;
        ty = pan.ty + e.clientY - pan.y;
        apply();
      }
    });

    const release = (e, cancelled) => {
      if (!pointers.delete(e.pointerId)) return;

      if (pinch && pointers.size >= 2) {
        pinch = { ...pair(), scale, tx, ty, home: pinch.home };
        return;
      }
      if (pinch && pointers.size === 1) {
        const [p] = pointers.values();
        pinch = null;
        pan = { x: p.x, y: p.y, tx, ty };
        return;
      }
      if (pointers.size > 0) return;

      pinch = pan = null;
      if (scale < 1.03) reset(true);
      const start = down;
      down = null;
      if (cancelled || multi || !start) return;

      const dx = e.clientX - start.x;
      const dy = e.clientY - start.y;

      if (Math.hypot(dx, dy) > 8) {
        suppressClick = true;
        setTimeout(() => (suppressClick = false));
        if (scale === 1 && !single && Math.abs(dx) > 60 && Math.abs(dx) > Math.abs(dy)) {
          show(index + (dx < 0 ? 1 : -1));
        }
        return;
      }

      const t = e.timeStamp;
      if (lastTap && t - lastTap.t < 300 && Math.hypot(e.clientX - lastTap.x, e.clientY - lastTap.y) < 30) {
        lastTap = null;
        if (scale > 1) reset(true);
        else if (e.target === img) zoomAt(e.clientX, e.clientY, DOUBLE_TAP_SCALE, true);
      } else {
        lastTap = { t, x: e.clientX, y: e.clientY };
      }
    };
    dialog.addEventListener("pointerup", (e) => release(e, false));
    dialog.addEventListener("pointercancel", (e) => release(e, true));

    dialog.addEventListener(
      "wheel",
      (e) => {
        e.preventDefault();
        const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 100 : 1;
        const dy = clamp(e.deltaY * unit, -80, 80);
        zoomAt(e.clientX, e.clientY, scale * Math.exp(-dy * (e.ctrlKey ? 0.008 : 0.0025)), true);
      },
      { passive: false },
    );

    for (const type of ["gesturestart", "gesturechange", "dragstart"]) {
      dialog.addEventListener(type, (e) => e.preventDefault());
    }
    img.draggable = false;
    window.addEventListener("resize", () => apply());

    document.addEventListener("click", (e) => {
      const t = e.target.closest(".thumb");
      if (t) open(thumbs.indexOf(t));
    });

    dialog.addEventListener("click", (e) => {
      if (suppressClick) return;
      if (e.target === dialog) return dialog.close();
      const action = e.target.closest("[data-action]")?.dataset.action;
      if (action === "close") dialog.close();
      if (action === "prev") show(index - 1);
      if (action === "next") show(index + 1);
    });

    dialog.addEventListener("keydown", (e) => {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      const [cx, cy] = [dialog.clientWidth / 2, dialog.clientHeight / 2];
      if (e.key === "+" || e.key === "=") zoomAt(cx, cy, scale * 1.5, true);
      else if (e.key === "-" || e.key === "_") zoomAt(cx, cy, scale / 1.5, true);
      else if (e.key === "0") reset(true);
      else if (!single && e.key === "ArrowLeft") show(index - 1);
      else if (!single && e.key === "ArrowRight") show(index + 1);
    });

    dialog.addEventListener("close", () => {
      reset();
      pointers.clear();
      pinch = pan = down = null;
      history.replaceState(null, "", location.pathname + location.search);
    });

    const openFromHash = () => {
      let id = "";
      try {
        id = decodeURIComponent(location.hash.slice(1));
      } catch {}
      const i = thumbs.findIndex((_, n) => idOf(n) === id);
      if (i !== -1) open(i);
      else if (dialog.open) dialog.close();
    };
    openFromHash();
    window.addEventListener("hashchange", openFromHash);
  }

  const track = document.querySelector(".carousel");
  const nav = document.querySelector(".carousel-nav");
  if (track && nav) {
    const prev = nav.querySelector('[data-carousel="prev"]');
    const next = nav.querySelector('[data-carousel="next"]');
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");

    const update = () => {
      const max = track.scrollWidth - track.clientWidth;
      nav.hidden = max < 4;
      track.classList.toggle("has-nav", max >= 4);
      prev.disabled = track.scrollLeft < 4;
      next.disabled = track.scrollLeft > max - 4;
    };
    const step = (dir) =>
      track.scrollBy({ left: dir * track.clientWidth * 0.8, behavior: reduced.matches ? "auto" : "smooth" });

    prev.addEventListener("click", () => step(-1));
    next.addEventListener("click", () => step(1));
    track.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
    update();
  }

  for (const btn of document.querySelectorAll("[data-copy]")) {
    btn.addEventListener("click", async () => {
      const label = btn.textContent;
      try {
        await navigator.clipboard.writeText(btn.dataset.copy);
        btn.textContent = "Copied";
      } catch {
        btn.textContent = "Copy manually";
        const range = document.createRange();
        range.selectNodeContents(btn.previousElementSibling);
        getSelection().removeAllRanges();
        getSelection().addRange(range);
      }
      setTimeout(() => (btn.textContent = label), 2000);
    });
  }
})();