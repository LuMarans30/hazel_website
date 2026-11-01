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

    document.addEventListener("click", (e) => {
      const t = e.target.closest(".thumb");
      if (t) open(thumbs.indexOf(t));
    });

    dialog.addEventListener("click", (e) => {
      if (e.target === dialog) return dialog.close();
      const action = e.target.closest("[data-action]")?.dataset.action;
      if (action === "close") dialog.close();
      if (action === "prev") show(index - 1);
      if (action === "next") show(index + 1);
    });

    dialog.addEventListener("keydown", (e) => {
      if (single) return;
      if (e.key === "ArrowLeft") show(index - 1);
      if (e.key === "ArrowRight") show(index + 1);
    });

    let startX = null;
    dialog.addEventListener("touchstart", (e) => (startX = e.touches[0].clientX), { passive: true });
    dialog.addEventListener("touchend", (e) => {
      if (startX === null || single) return;
      const dx = e.changedTouches[0].clientX - startX;
      startX = null;
      if (Math.abs(dx) > 60) show(dx < 0 ? index + 1 : index - 1);
    });

    dialog.addEventListener("close", () => {
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
