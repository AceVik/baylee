import { useCallback, useEffect, useState } from "react";

import { api, onSignedOut } from "./api";
import { Login } from "./Login";
import { ReportDetail } from "./ReportDetail";
import { ReportList } from "./ReportList";
import { navigate, useRoute } from "./router";

type Auth = { state: "checking" } | { state: "out" } | { state: "in"; name: string };

export function App() {
  const [auth, setAuth] = useState<Auth>({ state: "checking" });
  const route = useRoute();

  useEffect(() => {
    let live = true;
    api
      .me()
      .then((me) => {
        if (live) setAuth({ state: "in", name: me.name });
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
          setAuth({ state: "in", name });
        }}
      />
    );
  }
  return (
    <>
      <header className="bar">
        <a
          className="brand"
          href="/"
          onClick={(event) => {
            event.preventDefault();
            navigate("/");
          }}
        >
          Baylee reports
        </a>
        <span className="spacer" />
        <span className="muted who" title="Signed in as">
          {auth.name}
        </span>
        <button type="button" className="quiet" onClick={() => void signOut()}>
          Sign out
        </button>
      </header>
      {route.page === "report" ? (
        <ReportDetail id={route.id} key={route.id} />
      ) : (
        <ReportList search={route.search} />
      )}
    </>
  );
}
