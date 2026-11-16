/** What `fid dash --json` reports. Only the fields Studio reads are named. */
export interface Dash {
  product: { name: string; version: string; capabilities: string[]; guard_rules: number };
  git: {
    is_repo: boolean;
    branch: string | null;
    commit: string | null;
    subject: string | null;
    dirty_files: number;
    ahead: number;
    behind: number;
  };
  roadmap: { done: number; in_progress: number; todo: number; next: string | null };
  ci: {
    workflows: { file: string; name: string; triggers: string[]; checks_freshness: boolean }[];
    gates_declared_but_unrun: string[];
  };
  graph: { pipelines: { name: string; executor: string; outputs: string[] }[] };
  freshness: {
    artifacts_fresh: number;
    artifacts_stale: number;
    artifacts_missing: number;
    artifacts_untracked: number;
    problems: string[];
  };
  taxonomy: {
    declarations: { capability: string; name: string; kind: string; present: boolean }[];
  };
}

export interface DevStatus {
  running: boolean;
  command: string;
  port: number | null;
  url: string | null;
  log: string[];
}

export interface DeriveResult {
  ok: boolean;
  stdout: string;
  stderr: string;
}

/** A JSON Schema document, as `fid studio schema` emits it. */
export type Schema = Record<string, unknown>;

/**
 * Where Studio reads and writes a product.
 *
 * Two sources answer this: `fid studio` over loopback (files on disk) and
 * GitHub's API (a branch of a repository). Everything optional is something
 * only one of them can do; the UI asks `can` instead of asking which source it
 * has, so a third source needs no UI change.
 */
export interface Source {
  readonly kind: "local" | "github";
  /** Short label for the header: a path, or `owner/repo@branch`. */
  readonly label: string;
  files(): Promise<string[]>;
  read(path: string): Promise<string>;
  write(path: string, text: string, message?: string): Promise<void>;
  /** A declaration's JSON Schema: `fiducial` (default) or `topology`. */
  schema(name?: SchemaName): Promise<Schema>;
  dash?(): Promise<Dash>;
  derive?(check: boolean): Promise<DeriveResult>;
  /** Dev servers the source can run, by target: the site, or its content editor. */
  dev?: {
    status(target?: DevTarget): Promise<DevStatus>;
    start(target?: DevTarget): Promise<DevStatus>;
    stop(target?: DevTarget): Promise<DevStatus>;
  };
  /**
   * Changes collected on a branch for someone to review, instead of landing on
   * the base branch. Only a source that saves as pull requests has it.
   */
  review?: {
    /** The branch edits are going to, once there has been one. */
    branch(): string | null;
    /** Open a pull request for what has been saved; returns its URL. */
    open(title: string, body?: string): Promise<string>;
  };
}

export type DevTarget = "web" | "cms";
export type SchemaName = "fiducial" | "topology";
