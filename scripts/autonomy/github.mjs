import { invariant } from "./core.mjs";

const apiVersion = "2022-11-28";
const canonicalApiOrigin = "https://api.github.com";
const allowedMethods = new Set(["GET", "POST", "PATCH", "DELETE"]);

function issuePath(number) {
  invariant(
    Number.isInteger(number) && number > 0,
    "GitHub issue number must be a positive integer",
  );
  return String(number);
}

function commitPath(sha) {
  invariant(
    typeof sha === "string" && /^[0-9a-f]{40}$/u.test(sha),
    "GitHub commit SHA must be 40 lowercase hexadecimal characters",
  );
  return encodeURIComponent(sha);
}

export function buildGitHubApiUrl(endpoint) {
  invariant(
    typeof endpoint === "string" &&
      endpoint.startsWith("/") &&
      !endpoint.startsWith("//") &&
      !endpoint.includes("\\") &&
      !/[\u0000-\u001f\u007f]/u.test(endpoint),
    "unsafe GitHub API endpoint",
  );
  const rawPath = endpoint.split(/[?#]/u, 1)[0];
  for (const segment of rawPath.split("/")) {
    let decoded;
    try {
      decoded = decodeURIComponent(segment);
    } catch (error) {
      throw new Error("unsafe GitHub API endpoint", { cause: error });
    }
    invariant(
      !decoded.split(/[\\/]/u).some((part) => part === "." || part === ".."),
      "unsafe GitHub API endpoint",
    );
  }
  let url;
  try {
    url = new URL(endpoint, canonicalApiOrigin);
  } catch (error) {
    throw new Error("unsafe GitHub API endpoint", { cause: error });
  }
  invariant(
    url.protocol === "https:" &&
      url.origin === canonicalApiOrigin &&
      url.username === "" &&
      url.password === "" &&
      url.hash === "" &&
      !url.pathname.split("/").includes(".."),
    "GitHub API destination is outside the canonical origin",
  );
  return url;
}

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
  constructor({ repository, token, transport = globalThis.fetch }) {
    invariant(token, "GitHub token is required");
    invariant(typeof transport === "function", "GitHub transport is required");
    const { owner, name } = repositoryParts(repository);
    this.repository = repository;
    this.token = token;
    this.transport = transport;
    this.repoPath = `/repos/${owner}/${name}`;
  }

  async request(method, endpoint, body) {
    invariant(
      allowedMethods.has(method),
      `unsupported GitHub API method: ${method}`,
    );
    const requestUrl = buildGitHubApiUrl(endpoint);
    const requestBody = body === undefined ? undefined : JSON.stringify(body);
    // Network access is centralized here: the URL is canonicalized above and
    // redirects are disabled for every request.
    const response = await this.transport(requestUrl, {
      method,
      redirect: "error",
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${this.token}`,
        "Content-Type": "application/json",
        "X-GitHub-Api-Version": apiVersion,
        "User-Agent": "corpuslab-bounded-autonomy",
      },
      body: requestBody,
    });
    if (!response.ok) {
      let message = `GitHub API ${method} request failed with ${response.status}`;
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
    return this.request("GET", `${this.repoPath}/issues/${issuePath(number)}`);
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
    return this.paginate(
      `${this.repoPath}/issues/${issuePath(number)}/timeline?`,
    );
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
    return this.request(
      "POST",
      `${this.repoPath}/issues/${issuePath(number)}/labels`,
      { labels },
    );
  }

  async removeLabel(number, label) {
    const encoded = encodeURIComponent(label);
    try {
      await this.request(
        "DELETE",
        `${this.repoPath}/issues/${issuePath(number)}/labels/${encoded}`,
      );
    } catch (error) {
      if (!String(error.message).includes("failed with 404")) throw error;
    }
  }

  addComment(number, body) {
    return this.request(
      "POST",
      `${this.repoPath}/issues/${issuePath(number)}/comments`,
      { body },
    );
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
    return this.request(
      "GET",
      `${this.repoPath}/git/commits/${commitPath(sha)}`,
    );
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
