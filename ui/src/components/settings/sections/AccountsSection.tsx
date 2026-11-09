import { useState } from "react";
import { SectionHeader, TextInput } from "../controls";
import type { Account } from "../../../lib/accounts";

export type AccountsSectionProps = {
  accounts: Account[];
  activeId: string | null;
  onSignIn: (host: string) => void;
  onSignOut: (accountId: string) => void;
  onSetActive: (accountId: string) => void;
};

/** Accepts `example.com` or `https://example.com/path` and returns the host. */
function normalizeHost(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  try {
    const url = new URL(trimmed.includes("://") ? trimmed : `https://${trimmed}`);
    if (url.protocol !== "https:" || url.hostname === "github.com") return null;
    return url.host;
  } catch {
    return null;
  }
}

export default function AccountsSection({
  accounts,
  activeId,
  onSignIn,
  onSignOut,
  onSetActive,
}: AccountsSectionProps) {
  const [enterpriseHost, setEnterpriseHost] = useState("");
  const [error, setError] = useState<string | null>(null);

  const hasGithub = accounts.some((account) => account.host === "github.com");

  function signInEnterprise() {
    const host = normalizeHost(enterpriseHost);
    if (!host) {
      setError("Enter an HTTPS host, for example https://github.example.com.");
      return;
    }
    setError(null);
    onSignIn(host);
  }

  return (
    <>
      <SectionHeader
        title="Accounts"
        description="GitHub identities used for API actions. Tokens stay in the system keyring and are never shown here."
      />

      <h2 className="text-sm font-semibold">Connected accounts</h2>
      {accounts.length === 0 ? (
        <p className="mt-2 text-sm opacity-70">No accounts connected yet.</p>
      ) : (
        <ul className="mt-2 flex flex-col gap-2">
          {accounts.map((account) => (
            <li
              key={account.id}
              className="flex items-center gap-3 rounded border border-neutral-200 p-3 dark:border-neutral-800"
            >
              <img
                src={account.avatar_url}
                alt=""
                className="h-8 w-8 shrink-0 rounded-full bg-neutral-200 dark:bg-neutral-800"
              />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2 text-sm font-medium">
                  <span className="truncate">{account.login}</span>
                  {account.id === activeId && (
                    <span className="rounded bg-green-100 px-1.5 py-0.5 text-[10px] font-medium text-green-800 dark:bg-green-900 dark:text-green-200">
                      Active
                    </span>
                  )}
                </div>
                <div className="truncate text-xs opacity-60">{account.host}</div>
              </div>
              {account.id !== activeId && (
                <button
                  type="button"
                  onClick={() => onSetActive(account.id)}
                  className="rounded px-2 py-1 text-xs hover:bg-neutral-100 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
                >
                  Use
                </button>
              )}
              <button
                type="button"
                onClick={() => onSignOut(account.id)}
                className="rounded px-2 py-1 text-xs text-red-700 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-950 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
              >
                Sign out
              </button>
            </li>
          ))}
        </ul>
      )}

      <div className="mt-8">
        <h2 className="text-sm font-semibold">GitHub.com</h2>
        <p className="mt-1 text-xs opacity-70">
          Opens <span className="font-mono">github.com/login/device</span> with a
          one-time code. Knitt waits for authorization; no password is entered here.
        </p>
        <button
          type="button"
          onClick={() => onSignIn("github.com")}
          className="mt-3 rounded bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-800 dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
        >
          {hasGithub ? "Add another GitHub.com account" : "Sign in to GitHub.com"}
        </button>
      </div>

      <div className="mt-8">
        <h2 className="text-sm font-semibold">GitHub Enterprise</h2>
        <p className="mt-1 text-xs opacity-70">
          Only HTTPS hosts are accepted. Credentials are scoped to the host you enter.
        </p>
        <div className="mt-3 flex items-start gap-2">
          <TextInput
            value={enterpriseHost}
            onChange={(event) => setEnterpriseHost(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") signInEnterprise();
            }}
            placeholder="https://github.example.com"
            aria-label="GitHub Enterprise host"
            aria-invalid={error ? true : undefined}
            className="max-w-72"
          />
          <button
            type="button"
            onClick={signInEnterprise}
            className="shrink-0 rounded border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
          >
            Sign in
          </button>
        </div>
        {error && (
          <p role="alert" className="mt-2 text-xs text-red-700 dark:text-red-400">
            {error}
          </p>
        )}
      </div>
    </>
  );
}
