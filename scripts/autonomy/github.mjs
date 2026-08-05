import { invariant } from "./core.mjs";

const apiVersion = "2022-11-28";
const canonicalApiUrl = "https://api.github.com";

export function repositoryParts(repository) {
  invariant(
    /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u.test(repository),
    "GITHUB_REPOSITORY contains unsafe characters",
  );
  const [owner, name, extra] = repository.split("/");
  invariant(owner && name && !extra, "GITHUB_REPOSITORY must be owner/name");
  return { owner, name };
}

export class GitHubClient {
  constructor({ repository, token, apiUrl = "https://api.github.com" }) {
    invariant(token, "GitHub token is required");
    const normalizedApiUrl = apiUrl.replace(/\/$/, "");
    invariant(
      normalizedApiUrl === canonicalApiUrl,
      "autonomous publication is restricted to the canonical GitHub API",
    );
    repositoryParts(repository);
    this.repository = repository;
    this.token = token;
    this.apiUrl = normalizedApiUrl;
    this.repoPath = `/repos/${repository}`;
  }

  async request(method, endpoint, body) {
    invariant(
      ["GET", "POST", "PATCH", "DELETE"].includes(method),
      `unsupported GitHub API method: ${method}`,
    );
    invariant(
      endpoint.startsWith("/") &&
        !endpoint.includes("://") &&
        !endpoint.split(/[?#]/u, 1)[0].split("/").includes(".."),
      "unsafe GitHub API endpoint",
    );
    const requestUrl = `${canonicalApiUrl}${endpoint}`;
    const requestBody = body === undefined ? undefined : JSON.stringify(body);
    // Candidate blobs are intentionally published only to the canonical GitHub API.
    const response = await fetch(requestUrl, {
      method,
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${this.token}`,
        "X-GitHub-Api-Version": apiVersion,
        "User-Agent": "corpuslab-bounded-autonomy",
      },
      body: requestBody,
    });
    if (!response.ok) {
      let message = `GitHub API ${method} ${endpoint} failed with ${response.status}`;
      try {
        const payload = await response.json();
        if (typeof payload.message === "string")
          message += `: ${payload.message}`;
      } catch {
        // Keep the sanitized status-only message.
      }
      throw new Error(message);
    }
    if (response.status === 204) return undefined;
    return response.json();
  }

  async paginate(endpoint) {
    const separator = endpoint.includes("?") ? "&" : "?";
    const results = [];
    for (let page = 1; page <= 10; page += 1) {
      const batch = await this.request(
        "GET",
        `${endpoint}${separator}per_page=100&page=${page}`,
      );
      invariant(
        Array.isArray(batch),
        "paginated GitHub response must be an array",
      );
      results.push(...batch);
      if (batch.length < 100) return results;
    }
    throw new Error("GitHub pagination exceeded the bounded ten-page limit");
  }

  getIssue(number) {
    return this.request("GET", `${this.repoPath}/issues/${number}`);
  }

  listIssues(state = "open", labels = []) {
    const query = new URLSearchParams({
      state,
      sort: "updated",
      direction: "desc",
    });
    if (labels.length > 0) query.set("labels", labels.join(","));
    return this.paginate(`${this.repoPath}/issues?${query}`);
  }

  getTimeline(number) {
    return this.paginate(`${this.repoPath}/issues/${number}/timeline?`);
  }

  async collaboratorPermission(login) {
    const encoded = encodeURIComponent(login);
    const response = await this.request(
      "GET",
      `${this.repoPath}/collaborators/${encoded}/permission`,
    );
    return response.permission;
  }

  async hasOpenGeneratedPullRequest(label) {
    const query = encodeURIComponent(
      `repo:${this.repository} is:pr is:open label:"${label}"`,
    );
    const response = await this.request(
      "GET",
      `/search/issues?q=${query}&per_page=2`,
    );
    return response.total_count > 0;
  }

  addLabels(number, labels) {
    return this.request("POST", `${this.repoPath}/issues/${number}/labels`, {
      labels,
    });
  }

  async removeLabel(number, label) {
    const encoded = encodeURIComponent(label);
    try {
      await this.request(
        "DELETE",
        `${this.repoPath}/issues/${number}/labels/${encoded}`,
      );
    } catch (error) {
      if (!String(error.message).includes("failed with 404")) throw error;
    }
  }

  addComment(number, body) {
    return this.request("POST", `${this.repoPath}/issues/${number}/comments`, {
      body,
    });
  }

  createIssue({ title, body, labels }) {
    return this.request("POST", `${this.repoPath}/issues`, {
      title,
      body,
      labels,
    });
  }

  async upsertLabel(name, color, description) {
    const encoded = encodeURIComponent(name);
    try {
      await this.request("GET", `${this.repoPath}/labels/${encoded}`);
      return this.request("PATCH", `${this.repoPath}/labels/${encoded}`, {
        new_name: name,
        color,
        description,
      });
    } catch (error) {
      if (!String(error.message).includes("failed with 404")) throw error;
      return this.request("POST", `${this.repoPath}/labels`, {
        name,
        color,
        description,
      });
    }
  }

  getGitCommit(sha) {
    return this.request("GET", `${this.repoPath}/git/commits/${sha}`);
  }

  getRef(branch) {
    return this.request(
      "GET",
      `${this.repoPath}/git/ref/heads/${encodeURIComponent(branch)}`,
    );
  }

  createBlob(content) {
    return this.request("POST", `${this.repoPath}/git/blobs`, {
      content,
      encoding: "base64",
    });
  }

  createTree(baseTree, tree) {
    return this.request("POST", `${this.repoPath}/git/trees`, {
      base_tree: baseTree,
      tree,
    });
  }

  createCommit(message, tree, parent) {
    return this.request("POST", `${this.repoPath}/git/commits`, {
      message,
      tree,
      parents: [parent],
    });
  }

  createRef(branch, sha) {
    return this.request("POST", `${this.repoPath}/git/refs`, {
      ref: `refs/heads/${branch}`,
      sha,
    });
  }

  createPullRequest({ title, body, head, base = "main", draft = true }) {
    return this.request("POST", `${this.repoPath}/pulls`, {
      title,
      body,
      head,
      base,
      draft,
      maintainer_can_modify: true,
    });
  }
}

export function authorizedUsers(value) {
  return new Set(
    String(value || "MuneebHoda")
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean),
  );
}

export function labelNames(issue) {
  return (issue.labels ?? []).map((label) =>
    typeof label === "string" ? label : label.name,
  );
}

export async function actorCanAuthorize(client, login, allowlist) {
  if (!allowlist.has(login)) return false;
  const permission = await client.collaboratorPermission(login);
  return ["admin", "maintain", "write"].includes(permission);
}

export async function authorizedLabelEvent(
  client,
  issueNumber,
  label,
  allowlist,
) {
  const timeline = await client.getTimeline(issueNumber);
  const events = timeline
    .filter((event) => event.event === "labeled" && event.label?.name === label)
    .sort((left, right) =>
      String(right.created_at).localeCompare(String(left.created_at)),
    );
  if (events.length === 0 || !events[0].actor?.login) return undefined;
  return (await actorCanAuthorize(client, events[0].actor.login, allowlist))
    ? events[0]
    : undefined;
}
