// What the mirror is asked for, and by whom: one row per download, written off
// the response path, and the sums /api/downloads.json serves.
//
// The bucket is the only door to the release files (worker/download.ts), so a
// file it serves is a download this module can see. GitHub's own counter is the
// other half of the site's total, and the two never meet: a file served from the
// bucket is never fetched from GitHub, and a request that fell back there is
// GitHub's to count, not ours. What the bucket served (the counter's rows, read
// live) plus GitHub's numbers (which the "Refresh the download counts" workflow
// leaves in the bucket as manifests/downloads.json) is the whole of it.

/// <reference types="@cloudflare/workers-types" />

import { type Env } from "./api.ts";

/** The kinds of client worth telling apart. The user agent itself is never
 * stored — only which of these it was. */
const CLIENTS: [client: string, pattern: RegExp][] = [
  // The app's own updater (tauri-plugin-updater) fetching an update bundle.
  ["updater", /tauri|wry/i],
  // A terminal or a script: curl, wget, PowerShell's Invoke-WebRequest…
  ["script", /curl|wget|powershell|libwww|python|go-http|node|httpie|aria2|okhttp|java/i],
  // A browser — and the last test, because PowerShell calls itself Mozilla/5.0
  // too, and so do a few command-line tools.
  ["browser", /mozilla|chrome|safari|firefox|edg/i],
];

/** Which kind of client asked: "browser", "script", "updater", or "other" when
 * nothing here knows it. @param request */
export function clientOf(request: Request): string {
  const agent = request.headers.get("user-agent") ?? "";
  return CLIENTS.find(([, pattern]) => pattern.test(agent))?.[0] ?? "other";
}

/** One row for one download, in the background: a visitor is downloading a
 * file, not waiting for a count, and a database that refuses may never be the
 * reason a download fails — so this waits on nothing and swallows everything.
 * @param request @param env @param ctx @param tag @param name */
export function record(
  request: Request,
  env: Env,
  ctx: ExecutionContext | undefined,
  tag: string,
  name: string,
): void {
  try {
    // No binding (a deploy before it, a test without one): the download is the
    // only thing that matters.
    if (!env.STATS) return;
    const at = Date.now();
    const write = env.STATS
      .prepare("INSERT INTO downloads (at, day, tag, name, client, country) VALUES (?, ?, ?, ?, ?, ?)")
      // The day is UTC, and must stay equal to date(at / 1000, 'unixepoch').
      .bind(at, new Date(at).toISOString().slice(0, 10), tag, name, clientOf(request), request.cf?.country ?? null)
      .run()
      .then(
        () => {},
        () => {},
      );
    // waitUntil also keeps the isolate alive for a write the response has
    // already raced past.
    if (ctx) ctx.waitUntil(write);
    else void write;
  } catch {
    // Nothing here can be allowed to break a download.
  }
}

// --- the sums ---------------------------------------------------------------

/** The counter's rows, plus the dated object reads recovered from R2. Live
 * rows have their client class and country; history has neither, so it joins as
 * "unknown" with a null country. One scan of each table, one reduction. */
const COUNTED = `
  SELECT day, tag, name, client, country, SUM(n) AS n
  FROM (
    SELECT day, tag, name, client, country, 1 AS n FROM downloads
    UNION ALL
    SELECT day, tag, name, 'unknown' AS client, NULL AS country, downloads AS n FROM download_history
  )
  GROUP BY day, tag, name, client, country
`;

/** How many days the chart is given, dense and zero-filled: gaps in it would
 * read as days that never happened. */
const DAYS = 90;

/** How many files the by-file table lists, biggest first. The badge reads this
 * same document, and every file of every release is a long list to carry. */
const FILES = 20;

/** How many countries the by-country table lists. */
const COUNTRIES = 10;

/** How long an isolate keeps its answer before asking the database again. */
const TTL_MS = 60_000;

/** The last answer, failures included: a database that is down should not be
 * asked again on every request either. */
let remembered: { at: number; body: string } | null = null;

/** One row of the counter, as COUNTED returns it. */
interface Row {
  day: string;
  tag: string;
  name: string;
  client: string;
  country: string | null;
  n: number;
}

/** One file of the GitHub document, as scripts/site-manifest.ts writes it. */
interface CountedFile {
  platform: string | null;
  version: string;
  tag: string;
  name: string;
  count: number;
}

/** That document, as much of it as is read here. */
interface Counted {
  generatedAt: string | null;
  total: number;
  raw: number;
  files: CountedFile[];
}

/** What the counter's rows add up to, as the page is served it. */
interface Mirror {
  total: number;
  first: string | null;
  last: string | null;
  byDay: { day: string; count: number }[];
  byClient: { client: string; count: number }[];
  byCountry: { country: string; count: number }[];
}

/** Those sums, and the tallies the merged tables are built from. The tallies
 * stay in here: a Map reaches the page as an empty object. */
interface Rows {
  mirror: Mirror;
  /** By platform id (the updater's bundles included)… */
  platforms: Map<string | null, number>;
  /** …by version… */
  versions: Map<string, number>;
  /** …and by "/dl/" address — a release and a file — because the updater's
   * bundles are named without their version and belong to every release. */
  files: Map<string, number>;
}

/** The two halves side by side. A half that could not be read is null in every
 * row, never 0: nothing answered is not the same as nothing downloaded. */
interface Split {
  mirror: number | null;
  github: number | null;
  total: number;
}

const whole = (value: unknown) => (typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.floor(value) : 0);
const text = (value: unknown) => (typeof value === "string" ? value : "");

/** The GitHub document, if it is one at all: a missing or corrupt object is as
 * good as an empty one, and the mirror's half stands on its own. */
function counted(counts: Record<string, unknown> | null | undefined): Counted | null {
  if (!counts || counts.schema !== 1 || !Array.isArray(counts.files)) return null;
  const files: CountedFile[] = [];
  for (const file of counts.files as Record<string, unknown>[]) {
    const name = text(file?.name);
    if (!name) continue;
    files.push({
      platform: typeof file.platform === "string" ? file.platform : null,
      version: text(file.version),
      tag: text(file.tag),
      name,
      count: whole(file.count),
    });
  }
  return {
    generatedAt: typeof counts.generatedAt === "string" ? counts.generatedAt : null,
    total: whole(counts.total),
    raw: whole(counts.raw),
    files,
  };
}

/** The counter's rows, added up, or null when the database cannot answer. */
async function mirrored(env: Env, github: Counted | null): Promise<Rows | null> {
  let rows: Row[];
  try {
    const answer = await env.STATS.prepare(COUNTED).all<Row>();
    rows = answer.results ?? [];
  } catch {
    // No binding, or a database that cannot answer: the page says so and shows
    // GitHub's half alone.
    return null;
  }

  // The document is the only thing that knows what a file name is called
  // (downloads.json lists every file of every release); without it a name the
  // counter has is a name nothing here can label.
  const platform = new Map((github?.files ?? []).map((file) => [file.name, file.platform]));
  const days = new Map<string, number>();
  const platforms = new Map<string | null, number>();
  const versions = new Map<string, number>();
  const clients = new Map<string, number>();
  const countries = new Map<string, number>();
  const files = new Map<string, number>();
  let total = 0;
  let first: string | null = null;
  let last: string | null = null;

  for (const row of rows) {
    const n = whole(row.n);
    total += n;
    days.set(row.day, (days.get(row.day) ?? 0) + n);
    const where = platform.get(row.name) ?? null;
    platforms.set(where, (platforms.get(where) ?? 0) + n);
    const version = row.tag.replace(/^v/, "");
    versions.set(version, (versions.get(version) ?? 0) + n);
    clients.set(row.client, (clients.get(row.client) ?? 0) + n);
    files.set(`${row.tag}/${row.name}`, (files.get(`${row.tag}/${row.name}`) ?? 0) + n);
    if (row.country) countries.set(row.country, (countries.get(row.country) ?? 0) + n);
    if (!first || row.day < first) first = row.day;
    if (!last || row.day > last) last = row.day;
  }

  /** Biggest first. Two equal counts keep the order they were first seen in,
   * so the same numbers always draw the same table. */
  const rank = <T extends { count: number }>(tally: Map<string, number>, each: (key: string, count: number) => T): T[] =>
    [...tally].map(([key, count]) => each(key, count)).sort((a, b) => b.count - a.count);

  return {
    mirror: {
      total,
      first,
      last,
      byDay: dense(days),
      byClient: rank(clients, (client, count) => ({ client, count })),
      byCountry: rank(countries, (country, count) => ({ country, count })).slice(0, COUNTRIES),
    },
    platforms,
    versions,
    files,
  };
}

/** The last DAYS days, oldest first, with the days nothing happened on in
 * them. UTC, as the rows are: the days the page shows are the days counted. */
function dense(counts: Map<string, number>): { day: string; count: number }[] {
  const today = Date.parse(`${new Date().toISOString().slice(0, 10)}T00:00:00Z`);
  const days: { day: string; count: number }[] = [];
  for (let back = DAYS - 1; back >= 0; back--) {
    const day = new Date(today - back * 86_400_000).toISOString().slice(0, 10);
    days.push({ day, count: counts.get(day) ?? 0 });
  }
  return days;
}

/** One table out of the two halves: every key either half knows about, with
 * both halves' numbers and their sum. */
function split<T>(keys: Iterable<T>, mine: Map<T, number> | null, theirs: Map<T, number> | null): (Split & { key: T })[] {
  const rows = [...new Set(keys)].map((key) => {
    const mirror = mine ? (mine.get(key) ?? 0) : null;
    const github = theirs ? (theirs.get(key) ?? 0) : null;
    return { key, mirror, github, total: (mirror ?? 0) + (github ?? 0) };
  });
  return rows.sort((a, b) => b.total - a.total);
}

/** GitHub's own tally, by whatever the key is. */
function tally<K>(files: CountedFile[], of: (file: CountedFile) => K): Map<K, number> {
  const counts = new Map<K, number>();
  for (const file of files) counts.set(of(file), (counts.get(of(file)) ?? 0) + file.count);
  return counts;
}

/**
 * /api/downloads.json: what the bucket served and what GitHub counted, side by
 * side and added up. Either half may be missing — the document before its first
 * refresh, the database if it cannot answer — and then the other half is what
 * the page gets, with `sources` saying which it was.
 */
export async function statistics(env: Env, counts: Record<string, unknown> | null | undefined): Promise<string> {
  if (remembered && Date.now() - remembered.at < TTL_MS) return remembered.body;
  const github = counted(counts);
  const rows = await mirrored(env, github);
  const mirror = rows?.mirror ?? null;
  const halves = [mirror?.total, github?.total].filter((n): n is number => typeof n === "number");

  const platforms = split(
    [...(rows?.platforms.keys() ?? []), ...tally(github?.files ?? [], (file) => file.platform).keys()],
    rows?.platforms ?? null,
    github ? tally(github.files, (file) => file.platform) : null,
  );
  const versions = split(
    [...(rows?.versions.keys() ?? []), ...tally(github?.files ?? [], (file) => file.version).keys()],
    rows?.versions ?? null,
    github ? tally(github.files, (file) => file.version) : null,
  );

  // The files themselves, by their /dl/ address. The labels come from the
  // document, which lists every file of every release; a file of a release
  // GitHub no longer has is still a file the mirror served.
  const labels = new Map((github?.files ?? []).map((file) => [`${file.tag}/${file.name}`, file]));
  const files = split(
    [...(rows?.files.keys() ?? []), ...labels.keys()],
    rows?.files ?? null,
    github ? tally(github.files, (file) => `${file.tag}/${file.name}`) : null,
  )
    .slice(0, FILES)
    .map((row) => {
      const name = row.key.slice(row.key.indexOf("/") + 1);
      const label = labels.get(row.key);
      return {
        name,
        platform: label?.platform ?? null,
        version: label?.version ?? row.key.replace(/^v/, "").replace(/\/.*$/, ""),
        mirror: row.mirror,
        github: row.github,
        total: row.total,
      };
    });

  const body = JSON.stringify({
    schema: 1,
    generatedAt: new Date().toISOString(),
    total: halves.length ? halves.reduce((a, b) => a + b, 0) : null,
    mirror,
    github: github ? { total: github.total, raw: github.raw, updatedAt: github.generatedAt } : null,
    byPlatform: platforms.map(({ key, ...row }) => ({ platform: key, ...row })),
    byVersion: versions.map(({ key, ...row }) => ({ version: key, ...row })),
    byFile: files,
    sources: { mirror: mirror ? "d1" : "unavailable", github: github ? "r2" : "unavailable" },
  });
  remembered = { at: Date.now(), body };
  return body;
}
