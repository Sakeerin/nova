// nova doc: search and the theme toggle (spec
// docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md §10).
(function () {
  "use strict";
  var root = document.body.getAttribute("data-root") || "";
  var html = document.documentElement;

  function systemDark() {
    return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  }

  function currentTheme() {
    return html.getAttribute("data-theme") || (systemDark() ? "dark" : "light");
  }

  var toggle = document.getElementById("theme");
  if (toggle) {
    toggle.addEventListener("click", function () {
      var next = currentTheme() === "dark" ? "light" : "dark";
      html.setAttribute("data-theme", next);
      try {
        localStorage.setItem("nova-doc-theme", next);
      } catch (e) {
        // Storage may be off: the choice then lasts for this page.
      }
    });
  }

  var input = document.getElementById("search");
  var results = document.getElementById("results");
  var main = document.getElementById("main");
  if (!input || !results || !main) {
    return;
  }

  function rank(name, query) {
    var n = name.toLowerCase();
    if (n === query) {
      return 0;
    }
    if (n.indexOf(query) === 0) {
      return 1;
    }
    return n.indexOf(query) >= 0 ? 2 : -1;
  }

  function run() {
    var query = input.value.trim().toLowerCase();
    if (!query) {
      results.hidden = true;
      main.hidden = false;
      return;
    }
    var all = window.NOVA_SEARCH || [];
    var found = [];
    for (var i = 0; i < all.length; i++) {
      var r = rank(all[i][0], query);
      if (r >= 0) {
        found.push([r, i]);
      }
    }
    found.sort(function (a, b) {
      return a[0] - b[0] || a[1] - b[1];
    });
    results.textContent = "";
    var heading = document.createElement("h1");
    heading.textContent = found.length ? "Results" : "No results";
    results.appendChild(heading);
    var list = document.createElement("ul");
    for (var j = 0; j < found.length && j < 50; j++) {
      var e = all[found[j][1]];
      var li = document.createElement("li");
      var kind = document.createElement("span");
      kind.className = "kind";
      kind.textContent = e[1];
      var link = document.createElement("a");
      link.href = root + e[3];
      link.textContent = e[2];
      li.appendChild(kind);
      li.appendChild(link);
      if (e[4]) {
        var summary = document.createElement("div");
        summary.className = "summary";
        summary.textContent = e[4];
        li.appendChild(summary);
      }
      list.appendChild(li);
    }
    results.appendChild(list);
    main.hidden = true;
    results.hidden = false;
  }

  input.addEventListener("input", run);
  // A result on this page changes only the fragment, which would leave
  // the results shown: following any result clears the search.
  results.addEventListener("click", function (ev) {
    if (ev.target.closest("a")) {
      input.value = "";
      run();
    }
  });
  document.addEventListener("keydown", function (ev) {
    if (ev.key === "/" && document.activeElement !== input) {
      ev.preventDefault();
      input.focus();
    } else if (ev.key === "Escape" && document.activeElement === input) {
      input.value = "";
      run();
      input.blur();
    }
  });
})();
