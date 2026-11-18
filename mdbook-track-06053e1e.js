(() => {
    "use strict";

    const STORAGE_PREFIX = "mdbook-track::";
    const INPUT_SELECTOR =
        'input[data-track-item][data-track-book-id][data-track-page-id]';

    function storageKey(bookId) {
        return `${STORAGE_PREFIX}${bookId}`;
    }

    function emptyState() {
        return { version: 1, pages: {} };
    }

    function loadState(bookId) {
        try {
            const raw = window.localStorage.getItem(storageKey(bookId));
            if (!raw) return emptyState();

            const parsed = JSON.parse(raw);
            if (!parsed || typeof parsed !== "object") return emptyState();
            if (!parsed.pages || typeof parsed.pages !== "object") parsed.pages = {};
            if (!parsed.version) parsed.version = 1;
            return parsed;
        } catch (error) {
            console.warn("mdbook-track: unable to read progress", error);
            return emptyState();
        }
    }

    function saveState(bookId, state) {
        try {
            window.localStorage.setItem(storageKey(bookId), JSON.stringify(state));
        } catch (error) {
            console.warn("mdbook-track: unable to save progress", error);
        }
    }

    function pageState(state, pageId) {
        if (!state.pages[pageId] || typeof state.pages[pageId] !== "object") {
            state.pages[pageId] = { items: {} };
        }
        if (!state.pages[pageId].items || typeof state.pages[pageId].items !== "object") {
            state.pages[pageId].items = {};
        }
        return state.pages[pageId];
    }

    function trackedInputs() {
        return [...document.querySelectorAll(INPUT_SELECTOR)];
    }

    function inputsForItem(bookId, pageId, itemId) {
        return trackedInputs().filter(
            (box) =>
                box.dataset.trackBookId === bookId &&
                box.dataset.trackPageId === pageId &&
                box.dataset.trackItem === itemId,
        );
    }

    function itemIdsForTracker(tracker) {
        const declared = tracker.dataset.trackItems;
        if (declared) {
            return declared.split(",").map((id) => id.trim()).filter(Boolean);
        }

        return [...tracker.querySelectorAll("input[data-track-item]")]
            .map((box) => box.dataset.trackItem)
            .filter(Boolean);
    }

    function updateTrackerDisplay(tracker) {
        const bookId = tracker.dataset.bookId;
        const pageId = tracker.dataset.pageId;
        if (!bookId || !pageId) return;

        const ids = itemIdsForTracker(tracker);
        const state = loadState(bookId);
        const page = pageState(state, pageId);
        const completed = ids.filter((id) => page.items[id] === true).length;
        const count = tracker.querySelector(".mdbook-track__count");

        if (count) {
            count.textContent = `${completed} / ${ids.length} completed`;
        }

        tracker.classList.toggle(
            "mdbook-track--complete",
            ids.length > 0 && completed === ids.length,
        );
    }

    function updateAllTrackerDisplays(bookId = null, pageId = null) {
        for (const tracker of document.querySelectorAll(".mdbook-track")) {
            if (bookId && tracker.dataset.bookId !== bookId) continue;
            if (pageId && tracker.dataset.pageId !== pageId) continue;
            updateTrackerDisplay(tracker);
        }
    }

    function restoreInputs() {
        const states = new Map();

        for (const box of trackedInputs()) {
            const bookId = box.dataset.trackBookId;
            const pageId = box.dataset.trackPageId;
            const itemId = box.dataset.trackItem;
            if (!bookId || !pageId || !itemId) continue;

            if (!states.has(bookId)) states.set(bookId, loadState(bookId));
            const page = pageState(states.get(bookId), pageId);
            box.checked = page.items[itemId] === true;

            const item = box.closest(".mdbook-track__item");
            if (item) {
                item.classList.toggle("mdbook-track__item--complete", box.checked);
            }
        }

        updateAllTrackerDisplays();
    }

    function saveInput(box) {
        const bookId = box.dataset.trackBookId;
        const pageId = box.dataset.trackPageId;
        const itemId = box.dataset.trackItem;
        if (!bookId || !pageId || !itemId) return;

        const state = loadState(bookId);
        const page = pageState(state, pageId);
        page.items[itemId] = box.checked;
        saveState(bookId, state);

        for (const other of inputsForItem(bookId, pageId, itemId)) {
            other.checked = box.checked;
            const item = other.closest(".mdbook-track__item");
            if (item) {
                item.classList.toggle("mdbook-track__item--complete", other.checked);
            }
        }

        updateAllTrackerDisplays(bookId, pageId);
        refreshOverviews(bookId);
    }

    function initialiseInputs() {
        for (const box of trackedInputs()) {
            if (box.dataset.trackInitialised === "true") continue;
            box.dataset.trackInitialised = "true";
            box.addEventListener("change", () => saveInput(box));
        }
        restoreInputs();
    }

    function readManifest() {
        const node = document.querySelector("script.mdbook-track-manifest");
        if (!node) return null;

        try {
            return JSON.parse(node.textContent);
        } catch (error) {
            console.warn("mdbook-track: invalid manifest", error);
            return null;
        }
    }

    function statsForPage(page, state) {
        const storedItems = state.pages[page.id]?.items || {};
        const total = page.items.length;
        const completed = page.items.reduce(
            (count, item) => count + (storedItems[item.id] === true ? 1 : 0),
            0,
        );

        return {
            total,
            completed,
            complete: total > 0 && completed === total,
        };
    }

    function percentage(completed, total) {
        if (!total) return 0;
        return Math.max(0, Math.min(100, (completed / total) * 100));
    }

    function progressBar(className, completed, total, label) {
        const bar = document.createElement("div");
        bar.className = className;
        bar.setAttribute("role", "progressbar");
        bar.setAttribute("aria-valuemin", "0");
        bar.setAttribute("aria-valuemax", String(total));
        bar.setAttribute("aria-valuenow", String(completed));
        if (label) bar.setAttribute("aria-label", label);

        const fill = document.createElement("div");
        fill.className = `${className}-fill`;
        fill.style.setProperty(
            "--mdbook-track-progress",
            `${percentage(completed, total)}%`,
        );
        bar.appendChild(fill);
        return bar;
    }

    function bookRootUrl() {
        const script = [...document.scripts].find((node) => {
            if (!node.src) return false;
            try {
                const url = new URL(node.src, document.baseURI);
                return url.pathname.endsWith("/mdbook-track.js");
            } catch (_) {
                return false;
            }
        });

        if (script?.src) {
            return new URL(".", script.src);
        }

        return new URL(".", document.baseURI);
    }

    function chapterOutputPath(pageId) {
        if (!pageId) return "";

        // mdBook chapter source paths normally end in .md. Keep the logical
        // book-relative path, but point the overview at the generated HTML.
        return pageId.replace(/\.(?:md|markdown)$/i, ".html");
    }

    function chapterHref(page) {
        const outputPath = chapterOutputPath(page.id);
        if (!outputPath) return "#";
        return new URL(outputPath, bookRootUrl()).href;
    }

    function renderOverview(overview, manifest) {
        const bookId = overview.dataset.bookId;
        if (!bookId || !manifest || !Array.isArray(manifest.pages)) return;

        const state = loadState(bookId);
        const groups = new Map();
        let totalItems = 0;
        let completedItems = 0;
        let completedPages = 0;

        for (const page of manifest.pages) {
            const stats = statsForPage(page, state);
            totalItems += stats.total;
            completedItems += stats.completed;
            completedPages += stats.complete ? 1 : 0;

            const section = page.section || "Other";
            if (!groups.has(section)) groups.set(section, []);
            groups.get(section).push({ page, stats });
        }

        const content = overview.querySelector(".mdbook-track-overview__content");
        if (!content) return;
        content.replaceChildren();

        const heading = document.createElement("div");
        heading.className = "mdbook-track-overview__header";

        const title = document.createElement("strong");
        title.textContent = "Workbook progress";
        heading.appendChild(title);

        const overall = document.createElement("span");
        overall.className = "mdbook-track-overview__total";
        overall.textContent = `${completedPages} / ${manifest.pages.length} chapters · ${completedItems} / ${totalItems} items`;
        heading.appendChild(overall);
        content.appendChild(heading);

        content.appendChild(
            progressBar(
                "mdbook-track-overview__progress",
                completedItems,
                totalItems,
                "Workbook item completion",
            ),
        );

        for (const [section, entries] of groups) {
            const sectionNode = document.createElement("section");
            sectionNode.className = "mdbook-track-overview__section";

            let sectionTotal = 0;
            let sectionCompleted = 0;
            let sectionCompletedPages = 0;
            for (const { stats } of entries) {
                sectionTotal += stats.total;
                sectionCompleted += stats.completed;
                sectionCompletedPages += stats.complete ? 1 : 0;
            }

            const sectionHeader = document.createElement("div");
            sectionHeader.className = "mdbook-track-overview__section-header";

            const sectionTitle = document.createElement("h3");
            sectionTitle.className = "mdbook-track-overview__section-title";
            sectionTitle.textContent = section;

            const sectionCount = document.createElement("span");
            sectionCount.className = "mdbook-track-overview__section-count";
            sectionCount.textContent = `${sectionCompletedPages} / ${entries.length} chapters · ${sectionCompleted} / ${sectionTotal} items`;

            sectionHeader.append(sectionTitle, sectionCount);
            sectionNode.appendChild(sectionHeader);
            sectionNode.appendChild(
                progressBar(
                    "mdbook-track-overview__section-progress",
                    sectionCompleted,
                    sectionTotal,
                    `${section} item completion`,
                ),
            );

            const list = document.createElement("ul");
            list.className = "mdbook-track-overview__list";

            for (const { page, stats } of entries) {
                const entry = document.createElement("li");
                entry.className = "mdbook-track-overview__entry";

                const row = document.createElement("a");
                row.className = "mdbook-track-overview__row";
                row.href = chapterHref(page);
                row.setAttribute("aria-label", `${page.title}: ${stats.completed} of ${stats.total} items completed`);

                if (stats.complete) {
                    row.classList.add("mdbook-track-overview__row--complete");
                } else if (stats.completed > 0) {
                    row.classList.add("mdbook-track-overview__row--progress");
                } else {
                    row.classList.add("mdbook-track-overview__row--not-started");
                }

                const status = document.createElement("span");
                status.className = "mdbook-track-overview__status";
                status.setAttribute("aria-hidden", "true");
                status.style.setProperty(
                    "--mdbook-track-row-progress",
                    `${percentage(stats.completed, stats.total)}%`,
                );

                const label = document.createElement("span");
                label.className = "mdbook-track-overview__title";
                label.textContent = page.title;

                const count = document.createElement("span");
                count.className = "mdbook-track-overview__count";
                count.textContent = `${stats.completed} / ${stats.total}`;

                row.append(status, label, count);
                entry.appendChild(row);
                list.appendChild(entry);
            }

            sectionNode.appendChild(list);
            content.appendChild(sectionNode);
        }
    }

    function refreshOverviews(bookId = null) {
        const manifest = readManifest();
        if (!manifest) return;

        for (const overview of document.querySelectorAll(".mdbook-track-overview")) {
            if (!bookId || overview.dataset.bookId === bookId) {
                renderOverview(overview, manifest);
            }
        }
    }

    function initialise() {
        initialiseInputs();
        refreshOverviews();

        window.addEventListener("pageshow", () => {
            initialiseInputs();
            refreshOverviews();
        });

        window.addEventListener("storage", (event) => {
            if (event.key && event.key.startsWith(STORAGE_PREFIX)) {
                restoreInputs();
                refreshOverviews();
            }
        });
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", initialise, { once: true });
    } else {
        initialise();
    }
})();
