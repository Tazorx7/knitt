import { invoke } from "@tauri-apps/api/core";

/** Mirrors `auth::accounts::GitHubAccount` as returned by `auth_list_accounts`. */
export type Account = {
  id: string;
  host: string;
  login: string;
  name?: string;
  avatar_url: string;
};

export type AccountsResponse = {
  accounts: Account[];
  active_account_id: string | null;
};

export function listAccounts(): Promise<AccountsResponse> {
  return invoke<AccountsResponse>("auth_list_accounts");
}

export function signOutAccount(accountId: string): Promise<void> {
  return invoke("auth_sign_out", { accountId });
}

export function setActiveAccount(accountId: string): Promise<void> {
  return invoke("auth_set_active_account", { accountId });
}
