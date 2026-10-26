<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Proposal: Calls — REST and SOAP requests beside the code

**Prepared:** 2026-10-25  
**Target:** the author's to set (after 3.1).  
**Status:** Proposed. Not yet recorded. Six backlog items are suggested in [section 13](#13-backlog-items), FEAT-160 to FEAT-165, and drafts of their item documents come with this proposal, for `agile/items/`. Four questions wait for the author in [section 15](#15-questions-for-the-author).  
**Raised by:** the author, 2026-10-25: a new tab that is the idea of Postman, not Postman. It finds a collection file in the repository or takes one from outside; better, it scans the code's endpoints and lets the person choose which become calls. It should be a simple screen, easy to use and not cluttered; environments that are easy to manage; values from responses filled in automatically, and an easy way to put any response value into a variable; collections that are easy to compact and share, as JSON or a zip; everything configurable in Settings. Then, the same day: REST and SOAP both.  
**Screens:** 2C (Calls, new, at `/calls`), 1K (Settings › Calls, a new section), chrome (a rail row).  
**Designs:** [`design_handoff_calls/`](../../design_handoff_calls/), listed in [section 14](#14-designs).

## 1. Outcome

A person working on a service sends requests to it from the same window they review and commit in, and the requests live in the repository with the code they exercise.

- **Calls (2C)** is one screen: collections on the left, the request above the response in the middle, the active environment's variables on the right.
- **Keeping a value** is one click. Clicking any value in a response offers to keep it as a variable, once or on every call. A login's token reaches every later call without a script.
- **Import** turns code into calls. It reads Spring, JAX-RS and SOAP endpoints from source, and collection files from the repository or from outside, and the person ticks which ones to bring in.
- **Environments** are a short list with a colour each. Plain values are in the repository; secrets and kept values never are.
- **SOAP** is a first-class call: operations from the WSDL, an envelope built for you, WS-Security, XPath for kept values.
- **Share** is one dialog: one `.json`, a zip, or a commit.

## 2. Rules that shape the rest

1. **Nothing new is louder than the work.** One rail row, one route. The chrome is unchanged; where the artboards differ from `TitleBar.svelte`, `NavRail.svelte`, `Toolbar.svelte` and `StatusStrip.svelte`, the components win.
2. **Calls are files in the repository.** A collection is plain JSON under `.spagitty/calls/` that a person can read in a diff. Spagitty writes it with stable key order and two-space indent, so a change to one call is a change to a few lines.
3. **Secrets never reach the repository or a shared file.** A secret or kept value is written to the OS keychain, never to `.spagitty/`, a zip or an export. A test asserts it for every writer.
4. **The webview sends nothing.** Every request is made in Rust. The screen receives the response as data. This keeps the architecture's rule that the webview holds no HTTP client and no token ([section 10](#10-the-network-boundary)).
5. **No scripting engine.** Kept values and the few things people script for (a header from a variable, a timestamp) are declarative. Postman pre-request and test scripts are not run ([7.3](#73-postman)).
6. **Scanning reads; it never runs.** Import reads source files and specifications. It does not build, start or call the service.
7. **Labels only where they are needed.** Hover titles carry the detail (TASK-038, TASK-044).

## 3. Words used here

| Word | Means |
| --- | --- |
| **Call** | One saved request: REST (method, URL, headers, body) or SOAP (WSDL operation, endpoint, envelope). |
| **Collection** | A named set of calls in folders, one file under `.spagitty/calls/`. |
| **Environment** | A named set of variables with a colour, such as `local`, `staging` or `production`. One is active per repository. |
| **Variable** | `{{name}}` anywhere in a URL, header, body or envelope. |
| **Kept value** | A variable filled from a response by a rule: a call, a JSONPath or XPath, and *once* or *every call*. |
| **Secret** | A variable whose value lives in the keychain and is shown as dots. |
| **Source** | What Import reads: Spring controllers, SOAP services, an OpenAPI or WSDL file, or a collection file. |

## 4. Screens, codes and the rail

| Code | Screen | Route | Rail group | Replaces |
| --- | --- | --- | --- | --- |
| 2C | Calls | `/calls` | Repository, after Recipes | — |
| 1K | Settings › Calls | `/settings#calls` | — | — |

Import, Environments and Share are views of 2C (`/calls/import`, `/calls/environments`) and a dialog, not rail rows. The rail icon is in the artboards' scripts (`calls`: two opposed arrows). FEAT-134's rail settings gain a row for 2C, pinned by default.

## 5. Calls — the screen (FEAT-160)

### 5.1 Layout

- **Header**: the title, a chip per open call (method and name, each with ✕; middle-click closes too; `Ctrl+W` closes the current one), `+`, then the environment picker, *Import* and *Share*. Open calls are kept per repository.
- **Collections** (left): a filter (`Ctrl+K`), each collection with its file path and count, folders, and calls as method and name. A collapsed collection shows its kind (REST, SOAP) and count. Below them, a dashed card appears only when Import has found endpoints that are in no collection, and opens Import. *History* is at the foot.
- **The request** (middle, top): method, URL, *Send* (`Ctrl+Enter`). Tabs: *Params*, *Headers*, *Body*, *Auth*, *Keep*. A tab shows a count when it holds something. Body types: JSON, XML, form, multipart, raw, none.
- **The response** (middle, bottom): status with a dot, time and size; tabs *Body*, *Headers*, *Cookies*; *Beautify* ([5.4](#54-beautify)).
- **Variables** (right): the active environment's variables, kept values in `--ok` with where they came from. *Manage* opens Environments.

### 5.2 Variables in a call

- **`{{name}}`** is drawn as a pill wherever it appears. Hovering shows its value in the active environment (dots for a secret). Typing `{{` offers the names.
- **Resolution**: the active environment, then the variables shared by every environment, then the collection's own.
- **An unknown name** is drawn in `--danger`, and *Send* says which name is missing in which environment instead of sending a request with `{{x}}` in it.
- **Built-ins**, read-only and documented: `{{$timestamp}}`, `{{$isoNow}}`, `{{$uuid}}`, `{{$randomInt}}`.

### 5.3 Sending

- **The request is made in Rust** ([section 10](#10-the-network-boundary)) and is cancellable. While it runs, *Send* reads *Cancel*.
- **The response** is capped at 50 MB. Over 2 MB, the body is shown without colour. A binary body shows its type and size, and *Save as…*. `text/event-stream` is shown as events as they arrive.
- **History** records each send: the resolved request with secrets replaced by their names, the status, time and size, and the response up to 1 MB. It is in application data, per repository, capped by Settings ([section 9](#9-settings--calls-feat-165)).
- **An environment marked *Ask before sending*** asks once per call with the environment's name and colour. *production* is created with it on.

### 5.4 Beautify

- JSON and XML that arrive minified are shown formatted when *Beautify responses* is on (the default).
- The *Beautify* toggle switches the one response between formatted and as received. As received, values cannot be clicked to keep; the bar says so.
- Formatting is display only. *Copy* and *Save as…* give the bytes that arrived.

### 5.5 Storage

```
.spagitty/calls/
  orders-api.json          a collection
  billing-ws.json
  env/local.json           an environment's plain values
  env/staging.json
  shared.json              variables shared by every environment
  settings.json            this repository's overrides (section 9), optional
  examples/                saved responses, optional (section 8)
```

The collection format, version 1:

```json
{
  "v": 1,
  "name": "orders-api",
  "items": [
    { "folder": "Auth", "items": [
      { "id": "c_01H…", "name": "Log in", "kind": "rest",
        "method": "POST", "url": "{{baseUrl}}/auth/login",
        "headers": [["Accept", "application/json"]],
        "body": { "type": "json", "text": "{\n  \"email\": \"{{email}}\",\n  \"password\": \"{{password}}\"\n}" },
        "keep": [
          { "var": "token",  "from": "$.data.accessToken", "every": true },
          { "var": "userId", "from": "$.data.user.id",     "every": true }
        ] }
    ] }
  ]
}
```

A SOAP call ([section 7](#7-soap-feat-163)) has `"kind": "soap"` and `wsdl`, `service`, `port`, `operation`, `version`, `url`, `envelope` and `wsSecurity` in place of `method` and `body`.

An environment holds names for its secrets and kept values but never their values:

```json
{ "v": 1, "name": "staging", "color": "warn", "askBeforeSending": false,
  "vars": [
    { "name": "baseUrl",  "value": "https://staging.orders.local/api" },
    { "name": "email",    "value": "mira@example.com" },
    { "name": "password", "secret": true },
    { "name": "token",    "kept": true }
  ] }
```

**Per person, in application data** (not the repository): the active environment, open calls, history, the filter, and which collections are collapsed. **In the keychain**: secret and kept values, keyed by repository, environment and name, through the same seam as `forge/keychain.rs`.

## 6. Environments and kept values (FEAT-161)

### 6.1 Environments

- **The list**: each environment with its colour dot, host and *in use* on the active one. *+ New environment*. `Ctrl+E` switches from anywhere in Calls.
- **One environment**: its name, a colour from five (green, amber, red, blue, teal), *Ask before sending*, *Duplicate*. A table of variables (name, value, kind, kept in) with *+ Add variable*, then the variables shared by every environment.
- **Kinds**: *Plain* (in the repository), *Secret* (keychain, dots, revealed while the eye is held), *Kept* (filled by a rule; shows *every call* or *frozen*).
- **The footer** says where this environment lives: the repository file and the keychain.

### 6.2 Keeping a value

- **Clicking a value** in a formatted JSON or XML response selects it and opens the keep card inline under its line, never as a popover over the text. The card has: the path (JSONPath or XPath, computed from the click), *as* (a suggested name: the key, with `accessToken` becoming `token` and `user.id` becoming `userId`; editable), *in* (the active environment), *Update on every call*, *Cancel*, *Keep*.
- **Keep** writes the rule into the call's `keep` list, and the value into the keychain. The value is underlined dotted in `--ok` with `→ name` after it, and appears in the variables panel at once.
- **Clicking a kept value again** shows *Kept as {{name}} in staging · updates on every call* and *Forget*.
- **After every send**, each *every call* rule reads its path from the response. A path that finds nothing leaves the old value and marks the variable *stale* with the call's name.
- **Kept from responses** (in Environments) lists every rule in the environment with a switch. Off freezes the value.

## 7. SOAP (FEAT-163)

### 7.1 A SOAP collection

- **Made from a WSDL** (Import, or a WSDL file in the repository): one folder per service, one call per operation. The WSDL's path is kept, and *Rebuild from WSDL* rebuilds a call's envelope from it, keeping the values the person typed where an element still exists.
- **The request**: the SOAP version (1.1 or 1.2, from the WSDL when it says), the operation picker (*Service › Operation*), the endpoint URL (from the WSDL's address, with the host made a variable), *Send*. Tabs: *Envelope*, *Headers*, *WS-Security*, *Keep*.
- **The envelope** is generated with every element, a sample value, and a comment with its XSD type and whether it is required. Variables work as in REST.
- **Headers**: `SOAPAction` (1.1) or the `action` parameter of `Content-Type` (1.2) is set from the WSDL and shown, not hidden.
- **WS-Security**: *None*, *Username token* (user and password from variables, password text or digest, nonce and created) or *Signed* (an X.509 key from the keychain, a timestamp and a signed body).

### 7.2 The response

- **A SOAP fault** is shown as an error, with its code, string and detail, even when the server answered 200. The status line says *no fault* otherwise.
- **Keeping** works as in REST, with an XPath using the response's own prefixes (`//inv:GetInvoiceResponse/inv:invoice/inv:total`). The namespaces are stored with the rule.

### 7.3 Postman

- **Import** reads Postman collection v2.0 and v2.1 and Postman environments. Requests, folders, variables, auth and examples are brought in.
- **Scripts**: a test script line of the shape `pm.environment.set("x", pm.response.json().a.b)` or `pm.collectionVariables.set(…)` becomes a keep rule. Other script lines are kept in the call's `notes`, and Import lists how many calls have scripts that were not brought in.
- **Export** writes Postman 2.1. Keep rules become the same `pm.environment.set` lines, so the collection behaves the same in Postman. A SOAP call is exported as a raw POST with its headers and envelope.

## 8. Import (FEAT-162)

### 8.1 The screen

- **Sources** (left), in three groups. *Scan the code*: Spring controllers, SOAP services, an OpenAPI file. *Collection files in this repository*: whatever the scan of the tree found ([8.3](#83-files-in-the-repository)). *From outside*: *Choose a file…* and *Paste cURL*.
- **Endpoints** (middle): grouped by controller or service, with the file. Each row has a checkbox, the method, the path, a hint (the request class, the parameters, the media type), and a tag: *New*, *Changed* or *In <collection>*. A filter: *All*, *New*, *Changed*, *Already in*. *Scan again*.
- **Bring in** (right): *Calls in a collection* or *New collection*, *Into*, *Folders* (*One per controller* or *By path*), and three switches: *Example bodies from request classes*, *Path variables become variables*, *Server address from application.yml*. The button reads *Add 7 to orders-api* or *Create collection with 7*.
- **Defaults**: *New* and *Changed* rows are ticked; *Already in* rows are not.

### 8.2 Reading code

- **Java is parsed with tree-sitter.** The Java grammar is already compiled in for Files (FEAT-131). Import adds queries, not a parser.
- **Spring MVC and WebFlux**: `@RestController` and `@Controller` with `@ResponseBody`; `@RequestMapping` on the class and `@GetMapping`, `@PostMapping`, `@PutMapping`, `@PatchMapping`, `@DeleteMapping` and `@RequestMapping(method=…)` on methods. `@PathVariable`, `@RequestParam`, `@RequestHeader` and `@RequestBody` give the parameters.
- **JAX-RS**: `@Path`, `@GET` and the others, `@PathParam`, `@QueryParam`, `@Consumes` and `@Produces`.
- **SOAP**: `@WebService` and `@WebMethod` (JAX-WS), and Spring-WS `@Endpoint` with `@PayloadRoot`. Where a WSDL is in the repository or generated to a known path, it is used. Otherwise operations are listed from the code and the envelope is built from the request class.
- **Example bodies**: the `@RequestBody` type is found in the repository (a class or a record), and its fields become JSON with a sample per type, recursively to a depth of 4. Bean Validation annotations (`@NotNull`, `@Size`, `@Email`) shape the samples.
- **The server address**: `server.port` and `server.servlet.context-path` (or `spring.webflux.base-path`) from `application.yml` or `application.properties`, including the default profile, become `baseUrl` in a `local` environment when there is none.
- **What is read**: the working tree, read-only, so endpoints the person is writing now are found. `.gitignore`d paths and the Settings skip globs are skipped. The scan is in the background, streamed, cancellable, and capped at 20,000 files.

### 8.3 Files in the repository

- **Found by name and content**: `*.postman_collection.json`, `*.postman_environment.json`, Insomnia exports, Bruno folders (`bruno.json`), OpenAPI 3 and Swagger 2 (`openapi.*`, `swagger.*`, or a YAML or JSON with `openapi:`), `*.wsdl`, `*.http` and `*.rest`, `*.har`.
- **Opened in place**: with *Open collection files found in the repository* on, these are listed under *Found in this repository* and their calls can be sent without importing. They are never moved or rewritten. Keeping a value on one asks to import it first.

### 8.4 Changed and new

- **Matching** is by method and path with path variables normalised (`/orders/{id}` = `/orders/{{orderId}}`), and for SOAP by service and operation.
- **Changed** means the request shape differs: a parameter or body field was added or removed, or a type changed. The hint says what (*CreateOrderRequest gained couponCode*).
- **Bringing in a changed call** adds the new fields and removes the removed ones. Values the person typed are kept.

## 9. Settings › Calls (FEAT-165)

A new chip, *Calls*, after Behaviour, at `/settings#calls`, with its own branch in `routes/settings/+page.svelte` (see FEAT-134 on why there is no catch-all). A repository can override any setting in `.spagitty/calls/settings.json`; the section's line says so.

| Group | Setting | Default |
| --- | --- | --- |
| Where calls live | Folder in the repository | `.spagitty/calls/` |
| | Open collection files found in the repository | On |
| | Look for | Postman, Insomnia, Bruno, OpenAPI, .http, WSDL (HAR off) |
| Scanning code | Scan when a repository opens | Ask |
| | Frameworks | Spring MVC, Spring WebFlux, JAX-RS, JAX-WS, Spring-WS |
| | Skip paths | `**/test/**, **/generated/**` |
| Kept values | Click a response value to keep it | On |
| | A new kept value updates | On every call |
| | Suggest names | On |
| Sending | Time out after | 30 s |
| | Follow redirects | On |
| | Check TLS certificates | On (an environment may turn it off for itself) |
| | Proxy | System |
| | Client certificates | None (mutual TLS, by host) |
| | Beautify responses | On |
| | Header on every call | `User-Agent: Spagitty/<version>` |
| SOAP | Default version | 1.1 |
| | Build the envelope from the WSDL | On |
| | WS-Security | Username token |
| | Show a SOAP fault as an error | On |
| Safety and sharing | Secrets and kept values go in | System keychain |
| | Keep history | 500 calls |
| | Share as | Zip |
| | Leave secrets out when sharing | Always (not a choice) |

Micronaut, Quarkus, Express and FastAPI are drawn as off. They are a later item, and the chips are not built until a reader exists for each.

## 10. The network boundary

`docs/architecture.md` says `forge/` is the only part of the workspace that reaches a network, through exactly one `ureq` client in `forge/http.rs`, and the webview makes no request. Calls is a second, deliberate network surface, so the rule must change, and the change is the author's to accept ([question 1](#15-questions-for-the-author)).

The proposed shape keeps the promise as narrow as it can be:

- **`crates/spagitty-core/src/calls/`** is a new module. **`calls/send.rs`** is the only file outside `forge/http.rs` that may construct or use the HTTP client, and it uses the same `ureq`. No new HTTP client is linked.
- **The existing test** that asserts where the client is reachable is widened to exactly these two files, and still fails for a third.
- **`calls/send.rs` sends only what the person asked for**: a call they sent, or a WSDL or OpenAPI URL they typed into Import. Nothing in Calls reaches a network on its own: no telemetry, no update check, no fetching of remote `$ref`s unless the person says so.
- **The webview** still makes no request and holds no secret. It receives the response's status, headers and body as data, with secrets already replaced.
- **TLS** is rustls, as `forge/` uses. Client certificates and proxies are passed to `ureq`'s agent. HTTP/2 is not required; `ureq` speaks HTTP/1.1, and that is enough for this screen.

## 11. Dependencies

Each one is listed with the reason it is needed, as `AGENTS.md` asks. Before adding any, check `Cargo.lock`: some may already be there through another crate. Each licence must be checked against GPL-3.0-or-later and recorded in `NOTICE`.

| Dependency | Licence (to confirm) | For |
| --- | --- | --- |
| `ureq` (already in the workspace) | MIT / Apache-2.0 | Sending, from `calls/send.rs` only |
| `keyring` (already used by `forge/keychain.rs`, or the same seam) | MIT / Apache-2.0 | Secrets and kept values |
| `quick-xml` | MIT | SOAP envelopes, WSDL and XSD reading, XML formatting |
| `serde_json_path` | MIT | JSONPath for kept values |
| An XPath 1.0 evaluator (`sxd-document` with `sxd-xpath`, or one of its successors) | MIT / Apache-2.0 | XPath for kept values from XML and SOAP |
| A YAML reader (`serde_yaml_ng` or whichever the workspace already has) | MIT / Apache-2.0 | OpenAPI and `application.yml` |
| `zip` | MIT | Sharing as a zip and importing one |
| `tree-sitter-java` (already compiled in, FEAT-131) | MIT | Reading endpoints from Java |

No dependency is added to the frontend. The JSON and XML views reuse `VirtualRows` and the existing highlighter.

## 12. Performance

The author has said the UI is a bottleneck in large repositories. Calls may not make that worse:

- **Responses**: virtual rows for a body over 2,000 lines; colour only in view; no colour over 2 MB.
- **Import**: in the background, streamed, cancellable; the screen paints its sources before any endpoint is read.
- **No work on screens that are not open**: the scan runs only when Import is open or when Settings says *Always*, and then once per repository open.

## 13. Backlog items

Suggested numbers. A draft of each item document comes with this proposal, for `agile/items/`. The index rows for `agile/README.md` are in `design_handoff_calls/README.md`.

| ID | Title | Screen |
| --- | --- | --- |
| FEAT-160 | Calls: send a REST call and keep the collection in the repository | 2C |
| FEAT-161 | Environments, secrets and values kept from responses | 2C |
| FEAT-162 | Import calls from code and from collection files | 2C |
| FEAT-163 | SOAP calls from a WSDL | 2C |
| FEAT-164 | Share a collection as a file, a zip or a commit | 2C |
| FEAT-165 | Settings › Calls | 1K |

**Order of work**

1. **FEAT-160**: the screen, sending, the file format, history. It needs [question 1](#15-questions-for-the-author) answered first.
2. **FEAT-161**: environments, the keychain, keeping values. After this, Calls is useful every day.
3. **FEAT-165**: Settings › Calls, for the settings 160 and 161 already read.
4. **FEAT-162**: Import from Spring, JAX-RS, OpenAPI and collection files.
5. **FEAT-163**: SOAP. It reuses 161's keeping and 162's reading.
6. **FEAT-164**: Share.

## 14. Designs

All are in `design_handoff_calls/source/`, 1440×900, with dark as the reference theme (each artboard has a `theme` prop for light). PNGs of each are in `design_handoff_calls/screens/`.

| Artboard | Item | Shows |
| --- | --- | --- |
| `Main.dc.html` | FEAT-160, FEAT-161 | Calls: open calls with ✕, collections, a POST with variables, a JSON response with the keep card on `accessToken`, a kept `userId`, *Beautify*. **Interactive**: click a value, keep it, forget it; toggle *Beautify* |
| `Scan.dc.html` | FEAT-162 | Import from Spring controllers: sources, endpoints tagged New, Changed or In orders-api, *Bring in*. **Interactive**: tick rows, filter, switch *As* |
| `Environments.dc.html` | FEAT-161 | Environments: the list, staging's variables by kind and where they are kept, *Kept from responses*. **Interactive**: switch environments, *Ask before sending*, freeze a kept value |
| `SoapCall.dc.html` | FEAT-163 | A SOAP call from a WSDL: operation picker, generated envelope with WS-Security, an XML response with the keep card on an XPath |
| `Share.dc.html` | FEAT-164 | Share: one file, zip, or commit, what goes in, the preview. **Interactive**: every choice updates the preview and the button |
| `SettingsCalls.dc.html` | FEAT-165 | Settings › Calls, every group |

**When they disagree:** the spec wins on behaviour and the screens win on layout. Exact sizes and colours come from `metrics.ts` and `app.css`. The chrome is context, not design.

## 15. Questions for the author

1. **The network boundary.** Is Calls allowed to be a second network surface, as [section 10](#10-the-network-boundary) proposes (one more file allowed to use `ureq`, and the test widened to two)? Without this, Calls cannot be built.
2. **The name and the code.** *Calls* at 2C, so it does not read like *Pull requests* on the rail. Other names considered: *Requests* (clashes), *API*, *Endpoints*.
3. **Collections in the repository.** Recipes were kept out of `.spagitty/` because a file there shows as a working-copy change (FEAT-133, answer 6). Calls goes the other way on purpose: collections are meant to be committed and reviewed. Is `.spagitty/calls/` right, or should the folder be chosen by the repository (for example `http/` or `api/`)?
4. **GraphQL, gRPC and WebSocket.** Out of scope here. Is any of them wanted next?

## 16. Non-scope

- **Running scripts**: Postman pre-request and test scripts, or any JavaScript.
- **Tests and assertions**, collection runners, load tests, monitors and mocks.
- **GraphQL, gRPC, WebSocket** (see [question 4](#15-questions-for-the-author)).
- **Readers for other frameworks**: Micronaut, Quarkus, Express, FastAPI and others. Each is its own item.
- **Sync through a server or an account.** Sharing is files and git.
- **Starting the service** from Calls. Terminal (1T) is there for that.
