import { lazy, Suspense, useCallback, useEffect, useState, type MouseEvent } from "react";

import { BRAND, BrandMark } from "./Brand";
import { ContactList, ContactPage } from "./Contact";
import { api, onSignedOut } from "./api";
import { browserLang, t, type Lang } from "./i18n";
import { Login } from "./Login";
import { ReportDetail } from "./ReportDetail";
import { ReportList } from "./ReportList";
import { Toasts } from "./reports/pieces";
import { listPath, navigate, useRoute, type Route } from "./router";

// The admin console is most of the code and the fewest visits: it loads
// when it is first opened, so the reports come up sooner.
const Admin = lazy(async () => ({ default: (await import("./Admin")).Admin }));

type Auth = { state: "checking" } | { state: "out" } | { state: "in"; name: string; console: boolean };

/** A link's click as an in-page move; a modified click opens as usual. */
const go = (to: string) => (event: MouseEvent<HTMLAnchorElement>) => {
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) return;
  event.preventDefault();
  navigate(to);
};

/** Whether the page has scrolled past the top, for the header to fold. */
function useScrolled(): boolean {
  const [scrolled, setScrolled] = useState(false);
  useEffect(() => {
    const update = () => {
      setScrolled(window.scrollY > 8);
    };
    update();
    window.addEventListener("scroll", update, { passive: true });
    return () => {
      window.removeEventListener("scroll", update);
    };
  }, []);
  return scrolled;
}

/** The page's own name in the window's title; the console names its sections itself. */
function pageTitle(lang: Lang, route: Route): string | null {
  switch (route.page) {
    case "admin":
      return null;
    case "contact":
      return t(lang, "contact.title");
    case "report":
      return `${route.id.slice(0, 8)} · ${t(lang, "nav.reports")}`;
    default:
      return t(lang, "nav.reports");
  }
}


/** Where the skip link lands: the page's own `<main>`. */
function mainId(route: Route, console: boolean): string {
  if (route.page === "admin" && console) return "admin-main";
  if (route.page === "contact") return "contact-main";
  return "main";
}

export function App() {
  const [auth, setAuth] = useState<Auth>({ state: "checking" });
  const route = useRoute();
  const scrolled = useScrolled();
  const lang = browserLang();
  const signedIn = auth.state === "in";

  // The page says which language it is in (WCAG 3.1.1): the sign-in form is
  // English only, the rest the browser's.
  useEffect(() => {
    document.documentElement.lang = signedIn ? lang : "en";
  }, [lang, signedIn]);

  const title = signedIn ? pageTitle(lang, route) : null;
  useEffect(() => {
    if (!signedIn) document.title = BRAND;
    else if (title !== null) document.title = `${title} · ${BRAND}`;
  }, [signedIn, title]);

  useEffect(() => {
    let live = true;
    api
      .me()
      .then((me) => {
        if (live) setAuth({ state: "in", name: me.name, console: me.gateway_admin === true });
      })
      .catch(() => {
        // Signed out, or no answer: either way the way in is the form.
        if (live) setAuth({ state: "out" });
      });
    const stop = onSignedOut(() => {
      setAuth({ state: "out" });
    });
    return () => {
      live = false;
      stop();
    };
  }, []);

  const signOut = useCallback(async () => {
    try {
      await api.logout();
    } catch {
      // Already gone on the server; the page forgets it either way.
    }
    setAuth({ state: "out" });
  }, []);

  if (auth.state === "checking") {
    return (
      <main className="center" aria-busy="true">
        <p className="muted">Loading…</p>
      </main>
    );
  }
  if (auth.state === "out") {
    return (
      <Login
        onSignedIn={(name) => {
          // The sign-in answers the name; whether there is a console is
          // asked once more, as a reload would.
          setAuth({ state: "in", name, console: false });
          api
            .me()
            .then((me) => {
              setAuth({ state: "in", name: me.name, console: me.gateway_admin === true });
            })
            .catch(() => {
              // The list works without it.
            });
        }}
      />
    );
  }
  const onAdmin = route.page === "admin" && auth.console;
  const onContact = route.page === "contact";
  const onReports = !onAdmin && !onContact;
  return (
    <>
      <a className="skip-link" href={`#${mainId(route, auth.console)}`}>
        {t(lang, "nav.skip")}
      </a>
      <header className={scrolled ? "bar folded" : "bar"}>
        <a className="brand" href="/" onClick={go("/")}>
          <BrandMark />
          <span className="brand-long">{BRAND}</span>
          <span className="brand-short" aria-hidden="true">
            Baylee
          </span>
        </a>
        <nav className="tabs" aria-label={t(lang, "nav.label")}>
          <a href={listPath()} aria-current={onReports ? "page" : undefined} onClick={go(listPath())}>
            {t(lang, "nav.reports")}
          </a>
          <a href="/contact" aria-current={onContact ? "page" : undefined} onClick={go("/contact")}>
            {t(lang, "nav.contact")}
          </a>
          {auth.console && (
            <a href="/admin" aria-current={onAdmin ? "page" : undefined} onClick={go("/admin")}>
              {t(lang, "nav.admin")}
            </a>
          )}
        </nav>
        <span className="spacer" />
        <span className="who" title={t(lang, "nav.signedIn", { name: auth.name })}>
          <span className="avatar" aria-hidden="true">
            {[...auth.name][0] ?? "?"}
          </span>
          <span className="who-label">{auth.name}</span>
        </span>
        <button type="button" className="quiet" onClick={() => void signOut()}>
          {t(lang, "nav.signOut")}
        </button>
      </header>
      {onAdmin ? (
        <Suspense
          fallback={
            <main className="center" aria-busy="true">
              <p className="muted">{t(lang, "admin.loading")}</p>
            </main>
          }
        >
          <Admin
            lang={lang}
            section={route.page === "admin" ? route.section : "overview"}
            id={route.page === "admin" ? route.id : undefined}
            search={route.page === "admin" ? route.search : undefined}
          />
        </Suspense>
      ) : route.page === "contact" ? (
        route.id === undefined ? (
          <ContactList lang={lang} search={route.search} />
        ) : (
          <ContactPage key={route.id} lang={lang} id={route.id} />
        )
      ) : route.page === "report" ? (
        <ReportDetail id={route.id} key={route.id} lang={lang} me={auth.name} />
      ) : (
        <ReportList search={route.page === "list" ? route.search : ""} lang={lang} me={auth.name} />
      )}
      <Toasts lang={lang} />
    </>
  );
}
