import { useCallback, useEffect, useState, type MouseEvent } from "react";

import { Admin } from "./Admin";
import { api, onSignedOut } from "./api";
import { browserLang, t } from "./i18n";
import { Login } from "./Login";
import { ReportDetail } from "./ReportDetail";
import { ReportList } from "./ReportList";
import { listPath, navigate, useRoute } from "./router";

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

export function App() {
  const [auth, setAuth] = useState<Auth>({ state: "checking" });
  const route = useRoute();
  const scrolled = useScrolled();
  const lang = browserLang();

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
  return (
    <>
      <header className={scrolled ? "bar folded" : "bar"}>
        <a className="brand" href="/" onClick={go("/")}>
          <span className="brand-long">Baylee reports</span>
          <span className="brand-short" aria-hidden="true">
            Baylee
          </span>
        </a>
        {auth.console && (
          <nav className="tabs" aria-label={t(lang, "nav.label")}>
            <a href={listPath()} aria-current={onAdmin ? undefined : "page"} onClick={go(listPath())}>
              {t(lang, "nav.reports")}
            </a>
            <a href="/admin" aria-current={onAdmin ? "page" : undefined} onClick={go("/admin")}>
              {t(lang, "nav.admin")}
            </a>
          </nav>
        )}
        <span className="spacer" />
        <span className="muted who" title="Signed in as">
          {auth.name}
        </span>
        <button type="button" className="quiet" onClick={() => void signOut()}>
          {t(lang, "nav.signOut")}
        </button>
      </header>
      {onAdmin ? (
        <Admin
          lang={lang}
          section={route.page === "admin" ? route.section : "overview"}
          id={route.page === "admin" ? route.id : undefined}
          search={route.page === "admin" ? route.search : undefined}
        />
      ) : route.page === "report" ? (
        <ReportDetail id={route.id} key={route.id} />
      ) : (
        <ReportList search={route.page === "list" ? route.search : ""} />
      )}
    </>
  );
}
