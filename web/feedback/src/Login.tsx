import { useState, type FormEvent } from "react";

import { api, Refused } from "./api";

export function Login({ onSignedIn }: { onSignedIn: (name: string) => void }) {
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const me = await api.login(name.trim(), password);
      setPassword("");
      onSignedIn(me.name);
    } catch (e: unknown) {
      if (e instanceof Refused && e.status === 401) setError("Wrong name or password.");
      else if (e instanceof Refused && e.status === 429)
        setError("Too many failed sign-ins. Try again in a quarter of an hour.");
      else if (e instanceof Refused) setError(e.message);
      else setError("The service did not answer.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="center">
      <form className="card login" onSubmit={(event) => void submit(event)} aria-labelledby="login-title">
        <h1 id="login-title">Baylee reports</h1>
        <p className="muted">Admins only.</p>
        <label htmlFor="login-name">Name</label>
        <input
          id="login-name"
          name="username"
          autoComplete="username"
          autoCapitalize="none"
          spellCheck={false}
          required
          value={name}
          onChange={(event) => {
            setName(event.target.value);
          }}
        />
        <label htmlFor="login-password">Password</label>
        <input
          id="login-password"
          name="password"
          type="password"
          autoComplete="current-password"
          required
          value={password}
          onChange={(event) => {
            setPassword(event.target.value);
          }}
        />
        {error !== null && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <button type="submit" className="primary" disabled={busy}>
          {busy ? "Signing in…" : "Sign in"}
        </button>
      </form>
    </main>
  );
}
