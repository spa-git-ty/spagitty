// SPDX-License-Identifier: GPL-3.0-or-later

//! Recipes, as the screen reaches them (FEAT-133).
//!
//! The rules are `spagitty_core::recipes`: the blocks, the validator, the
//! brief, the restore point and the run. This module is what they need from
//! the application: where recipes are kept, what is true of the repository
//! that git does not know, the agent, and a thread for a run to be on.
//!
//! # What is kept where
//!
//! In the data directory, under `recipes/<repository>/`, where `<repository>`
//! is a hash of its path:
//!
//! - `recipes.json`: the recipes made for that repository. Never under
//!   `.spagitty/` in the repository, where a file is a working-copy change.
//! - `runs/<id>.json`: each run's record — the recipe as it was, every
//!   command and its output, every stop and what was chosen, and the restore
//!   point. Never sent anywhere.
//!
//! # The agent
//!
//! Only with a recipe's *Agent help* on, and only asked: for a stack of
//! blocks, for one line of a commit message, for what went wrong. It runs no
//! command and is never started in the working copy. With the help off this
//! module does not call it at all.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use spagitty_core::recipes::brief::{self, About};
use spagitty_core::recipes::restore::{self, RestorePoint};
use spagitty_core::recipes::run::{self, Checked, Choice, Failure, Host, Line, Run, Wanted};
use spagitty_core::recipes::validate::{self, Finding, World};
use spagitty_core::recipes::{command, propose, script, world, Level, Recipe, Step, VERSION};
use spagitty_farm::assign::level::Level as FarmLevel;
use spagitty_farm::assign::record::fnv;
use spagitty_farm::persistence::store::load_farm;
use spagitty_farm::verification::command::{run_cancellable, TIMEOUT};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::commands::AppState;

/// A run changed: a step ran, it stopped, it ended. Carries the whole run.
pub const EVENT: &str = "recipe-run";
/// A line was added to a run's output, as it happened.
pub const LINE: &str = "recipe-line";
/// How many runs' records are kept for a repository.
const RECORDS: usize = 20;

type Result<T> = std::result::Result<T, String>;

/// A run that is on a thread.
struct Live {
    repo: PathBuf,
    run: Mutex<Run>,
    /// Wakes the thread when the person has answered.
    wake: Mutex<Sender<()>>,
    /// The person pressed *Stop*. Shared with the host, where it also ends
    /// a check that is running.
    stop: Arc<AtomicBool>,
    /// A step is running right now.
    working: AtomicBool,
    /// What the screen was last sent. `run` is held for as long as a step
    /// takes, and a check can take twenty minutes: a screen that asks what
    /// the run is doing is given this, and does not wait for the step.
    last: Mutex<Option<RunView>>,
}

#[derive(Default)]
pub struct RecipesState {
    /// One run at a time for each repository, by its path.
    live: Mutex<HashMap<String, Arc<Live>>>,
}

pub fn manage<R: Runtime>(app: &AppHandle<R>) {
    app.manage(RecipesState::default());
}

fn key_of(repo: &Path) -> String {
    repo.to_string_lossy().into_owned()
}

/// The folder a repository's recipes and records are in.
fn folder<R: Runtime>(app: &AppHandle<R>, repo: &Path) -> Result<PathBuf> {
    let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(data
        .join("recipes")
        .join(format!("{:016x}", fnv(key_of(repo).as_bytes()))))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Saved {
    recipes: Vec<Recipe>,
}

/// The recipes in `folder`. A file that is not there is none; one that
/// cannot be read is an error, so it is not written over.
pub fn read_recipes(folder: &Path) -> Result<Vec<Recipe>> {
    let file = folder.join("recipes.json");
    match std::fs::read(&file) {
        Ok(bytes) => serde_json::from_slice::<Saved>(&bytes)
            .map(|saved| saved.recipes)
            .map_err(|e| {
                format!(
                    "{} could not be read, so it was left as it is: {e}",
                    file.display()
                )
            }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}

fn write_json(file: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let partial = file.with_extension("json.partial");
    std::fs::write(&partial, json).map_err(|e| e.to_string())?;
    std::fs::rename(partial, file).map_err(|e| e.to_string())
}

pub fn write_recipes(folder: &Path, recipes: &[Recipe]) -> Result<()> {
    write_json(
        &folder.join("recipes.json"),
        &Saved {
            recipes: recipes.to_vec(),
        },
    )
}

/// Put `recipe` among `recipes`: in the place of the one called `was`, or of
/// one with its own name, or at the end. Refused when the name is another
/// recipe's.
pub fn put(recipes: &mut Vec<Recipe>, mut recipe: Recipe, was: Option<&str>) -> Result<()> {
    recipe.name = recipe.name.trim().to_string();
    recipe.v = VERSION;
    if recipe.name.is_empty() {
        return Err("Name the recipe.".into());
    }
    let place = was
        .and_then(|was| recipes.iter().position(|saved| saved.name == was))
        .or_else(|| recipes.iter().position(|saved| saved.name == recipe.name));
    let taken = recipes
        .iter()
        .enumerate()
        .any(|(at, saved)| saved.name == recipe.name && Some(at) != place);
    if taken {
        return Err(format!("There is already a recipe called {}.", recipe.name));
    }
    match place {
        Some(at) => recipes[at] = recipe,
        None => recipes.push(recipe),
    }
    Ok(())
}

/// A recipe from the JSON it was exported as.
pub fn import(text: &str) -> Result<Recipe> {
    let recipe: Recipe =
        serde_json::from_str(text.trim()).map_err(|e| format!("That is not a recipe: {e}"))?;
    if recipe.v != VERSION {
        return Err(format!(
            "That recipe is version {}. This Spagitty reads version {VERSION}.",
            recipe.v
        ));
    }
    Ok(recipe)
}

fn level_of(level: FarmLevel) -> Level {
    match level {
        FarmLevel::Suggest => Level::Suggest,
        FarmLevel::StepByStep => Level::StepByStep,
        FarmLevel::SignOff => Level::SignOff,
        FarmLevel::Unattended => Level::Unattended,
    }
}

/// The branches a recipe could push to that are on the never-unattended
/// list: the repository's own, and the ones the recipe makes.
pub fn protected(
    branches: &[String],
    recipe: &Recipe,
    may_land_unattended: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut names: Vec<String> = branches.to_vec();
    names.extend(recipe.steps.iter().filter_map(|step| match step {
        Step::NewBranch { name, .. } => Some(name.clone()),
        _ => None,
    }));
    names.sort();
    names.dedup();
    names.retain(|name| !may_land_unattended(name));
    names
}

/// What is true of the repository: what git knows, and what only the
/// application does.
fn world_for<R: Runtime>(app: &AppHandle<R>, repo: &Path, recipe: &Recipe) -> Result<World> {
    let mut world = world::gather(repo, recipe).map_err(|e| e.to_string())?;
    let rules = crate::agents::rules_of(app, &key_of(repo));
    world.highest = level_of(rules.highest);
    world.protected = protected(&world.branches, recipe, |name| {
        rules.may_land_unattended(name)
    });
    world.checks = load_farm(repo)
        .map(|farm| farm.verification)
        .unwrap_or_default()
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect();
    world.agents = crate::agents::askable(app, &key_of(repo));
    Ok(world)
}

/// The brief for `recipe`, when anybody is briefed.
fn brief_for<R: Runtime>(
    app: &AppHandle<R>,
    repo: &Path,
    recipe: &Recipe,
    world: &World,
    prune: bool,
) -> Option<String> {
    if !recipe.agent_help {
        return None;
    }
    let agent = recipe
        .agent
        .as_deref()
        .and_then(|id| crate::agents::name_of(app, id))
        .unwrap_or_else(|| "(no agent chosen)".into());
    let person = spagitty_core::repo::open(repo)
        .ok()
        .and_then(|opened| {
            opened
                .config_snapshot()
                .string("user.name")
                .map(|name| name.to_string())
        })
        .unwrap_or_default();
    let name = repo
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Some(brief::brief(
        recipe,
        &About {
            repo: &name,
            branch: world.current.as_deref().unwrap_or_default(),
            agent: &agent,
            level: recipe.level.min(world.highest),
            person: &person,
        },
        prune,
    ))
}

// ── Recipes ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listed {
    pub recipes: Vec<Recipe>,
    /// `recipes.md`'s own version.
    pub guide_version: u32,
}

#[tauri::command(async)]
pub fn recipes_list<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Result<Listed> {
    let repo = state.open_path("recipes_list").map_err(|e| e.to_string())?;
    Ok(Listed {
        recipes: read_recipes(&folder(&app, &repo)?)?,
        guide_version: brief::guide_version(),
    })
}

/// Save a recipe. `was` is the name it was saved under before, when it has
/// been renamed.
#[tauri::command(async)]
pub fn recipes_save<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    recipe: Recipe,
    was: Option<String>,
) -> Result<Vec<Recipe>> {
    let repo = state.open_path("recipes_save").map_err(|e| e.to_string())?;
    let folder = folder(&app, &repo)?;
    let mut recipes = read_recipes(&folder)?;
    put(&mut recipes, recipe, was.as_deref())?;
    write_recipes(&folder, &recipes)?;
    Ok(recipes)
}

#[tauri::command(async)]
pub fn recipes_delete<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<Recipe>> {
    let repo = state
        .open_path("recipes_delete")
        .map_err(|e| e.to_string())?;
    let folder = folder(&app, &repo)?;
    let mut recipes = read_recipes(&folder)?;
    recipes.retain(|saved| saved.name != name);
    write_recipes(&folder, &recipes)?;
    Ok(recipes)
}

#[tauri::command]
pub fn recipes_import(text: String) -> Result<Recipe> {
    import(&text)
}

/// Everything the builder shows about a stack, in one answer.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub findings: Vec<Finding>,
    /// How many of them stop a run.
    pub errors: usize,
    /// Each step in a sentence.
    pub labels: Vec<String>,
    /// Each step's command, as it will be run.
    pub commands: Vec<String>,
    /// What the agent is given. `None` with its help off.
    pub brief: Option<String>,
    pub script: String,
    /// The level it would run at, after the repository's cap.
    pub level: Level,
    pub world: World,
}

/// Validate a stack against the repository as it is, and say what it is.
/// Run on every change, and again at *Run*.
#[tauri::command(async)]
pub fn recipes_check<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    recipe: Recipe,
) -> Result<Check> {
    let repo = state
        .open_path("recipes_check")
        .map_err(|e| e.to_string())?;
    let prune = crate::settings::load(&app).prune_on_fetch;
    let world = world_for(&app, &repo, &recipe)?;
    let findings = validate::validate(&recipe, &world);
    let start = world.current.clone().unwrap_or_default();
    Ok(Check {
        errors: validate::errors(&findings),
        findings,
        labels: (0..recipe.steps.len())
            .map(|index| recipe.describe(index, &start))
            .collect(),
        commands: recipe
            .steps
            .iter()
            .map(|step| command::shown(step, prune))
            .collect(),
        brief: brief_for(&app, &repo, &recipe, &world, prune),
        script: script::script(&recipe, &start, prune),
        level: recipe.level.min(world.highest),
        world,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposed {
    pub steps: Vec<Step>,
    /// What the agent said about them.
    pub said: String,
}

/// Ask the recipe's agent for a stack of blocks from a sentence. What comes
/// back is for the builder: nothing of it has run, or will until *Run*.
#[tauri::command(async)]
pub fn recipes_propose<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    recipe: Recipe,
    sentence: String,
) -> Result<Proposed> {
    if !recipe.agent_help {
        return Err("Agent help is off.".into());
    }
    let agent = recipe
        .agent
        .clone()
        .ok_or_else(|| "Choose an agent first.".to_string())?;
    let sentence = propose::sentence_ok(&sentence)?.to_string();
    let repo = state
        .open_path("recipes_propose")
        .map_err(|e| e.to_string())?;
    let world = world_for(&app, &repo, &recipe)?;
    let said = crate::agents::ask(
        &app,
        &key_of(&repo),
        &agent,
        "You answer one question about a Spagitty recipe. You run no command and edit no file.",
        &propose::prompt(&sentence, &world),
    )?;
    let proposal = propose::read(&said)?;
    Ok(Proposed {
        steps: proposal.steps,
        said: proposal.said,
    })
}

// ── A run ────────────────────────────────────────────────────────────────

/// A run as the screen is sent it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunView {
    pub repo: String,
    pub run: Run,
    /// It is on a thread: it can be answered and stopped. False for a record
    /// read back after Spagitty was closed.
    pub live: bool,
    /// A step is running right now.
    pub working: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LineEvent<'a> {
    repo: &'a str,
    id: &'a str,
    line: &'a Line,
}

/// What a run needs that is not git: the screen, the checks and the agent.
struct AppHost<R: Runtime> {
    app: AppHandle<R>,
    key: String,
    id: String,
    /// The agent to ask, and what it is told first. `None` with the help off.
    agent: Option<(String, String)>,
    stop: Arc<AtomicBool>,
}

impl<R: Runtime> Host for AppHost<R> {
    fn say(&mut self, line: &Line) {
        let _ = self.app.emit(
            LINE,
            LineEvent {
                repo: &self.key,
                id: &self.id,
                line,
            },
        );
    }

    fn check(&mut self, repo: &Path, line: &str) -> Checked {
        let ran = run_cancellable(repo, line, &self.stop, TIMEOUT);
        Checked {
            passed: ran.passed,
            output: ran.output,
        }
    }

    fn message(&mut self, _repo: &Path, wanted: &Wanted) -> std::result::Result<String, String> {
        let (agent, brief) = self
            .agent
            .as_ref()
            .ok_or_else(|| "Agent help is off.".to_string())?;
        crate::agents::ask(
            &self.app,
            &self.key,
            agent,
            brief,
            &format!(
                "## Now\nWrite the message for step {}: {}.\n\nThe changes it is about:\n\n{}\n\nPut the one line between <message> and </message>.",
                wanted.step + 1,
                wanted.what,
                wanted.changes
            ),
        )
    }

    fn explain(&mut self, _repo: &Path, failure: &Failure) -> Option<String> {
        let (agent, brief) = self.agent.as_ref()?;
        let said = crate::agents::ask(
            &self.app,
            &self.key,
            agent,
            brief,
            &format!(
                "## Now\nStep {} stopped: {}.\n\nCommand: {}\nExit code: {}\nOutput:\n{}\n\n`git status --short`:\n{}\n\nSay what went wrong and what the person can do, between <explanation> and </explanation>.",
                failure.step + 1,
                failure.what,
                failure.command,
                failure
                    .code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "none".into()),
                failure.output,
                failure.status
            ),
        )
        .ok()?;
        run::explanation_from(&said)
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
}

fn record_file(folder: &Path, id: &str) -> PathBuf {
    folder.join("runs").join(format!("{id}.json"))
}

/// The records in `folder`, newest first by the time in their names.
fn records(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder.join("runs"))
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files.reverse();
    files
}

/// Write a run's record, and let the oldest go past [`RECORDS`].
pub fn keep_record(folder: &Path, run: &Run) -> Result<()> {
    write_json(&record_file(folder, &run.id), run)?;
    for old in records(folder).into_iter().skip(RECORDS) {
        let _ = std::fs::remove_file(old);
    }
    Ok(())
}

/// The newest record in `folder`, if it can be read.
pub fn last_record(folder: &Path) -> Option<Run> {
    records(folder)
        .into_iter()
        .find_map(|file| serde_json::from_slice(&std::fs::read(file).ok()?).ok())
}

fn view(live: &Live, run: &Run) -> RunView {
    RunView {
        repo: key_of(&live.repo),
        run: run.clone(),
        live: true,
        working: live.working.load(Ordering::SeqCst),
    }
}

/// Keep the record and tell the screen.
fn publish<R: Runtime>(app: &AppHandle<R>, live: &Live) -> RunView {
    let run = live.run.lock().unwrap_or_else(|p| p.into_inner());
    if let Ok(folder) = folder(app, &live.repo) {
        let _ = keep_record(&folder, &run);
    }
    let view = view(live, &run);
    remember(live, &view);
    let _ = app.emit(EVENT, &view);
    view
}

/// Keep `view` as what the screen was last sent.
fn remember(live: &Live, view: &RunView) {
    *live.last.lock().unwrap_or_else(|p| p.into_inner()) = Some(view.clone());
}

fn host_for<R: Runtime>(app: &AppHandle<R>, live: &Arc<Live>, run: &Run) -> AppHost<R> {
    let prune = crate::settings::load(app).prune_on_fetch;
    let agent = run
        .recipe
        .agent_help
        .then(|| run.recipe.agent.clone())
        .flatten()
        .map(|agent| {
            let world = World {
                current: (!run.branch.is_empty()).then(|| run.branch.clone()),
                highest: run.level,
                ..World::default()
            };
            let brief = brief_for(app, &live.repo, &run.recipe, &world, prune).unwrap_or_default();
            (agent, brief)
        });
    AppHost {
        app: app.clone(),
        key: key_of(&live.repo),
        id: run.id.clone(),
        agent,
        stop: live.stop.clone(),
    }
}

/// The run's thread: a step, then the next, waiting where it stops.
fn drive<R: Runtime>(app: AppHandle<R>, live: Arc<Live>, woken: Receiver<()>) {
    loop {
        let (more, over) = {
            let mut run = live.run.lock().unwrap_or_else(|p| p.into_inner());
            let mut host = host_for(&app, &live, &run);
            live.working.store(true, Ordering::SeqCst);
            let starting = view(&live, &run);
            remember(&live, &starting);
            let _ = app.emit(EVENT, &starting);
            let more = run.tick(&live.repo, &mut host);
            live.working.store(false, Ordering::SeqCst);
            (more, run.is_over())
        };
        publish(&app, &live);
        if over {
            return;
        }
        if !more && woken.recv().is_err() {
            return;
        }
    }
}

fn live_for(state: &RecipesState, repo: &Path) -> Option<Arc<Live>> {
    state
        .live
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&key_of(repo))
        .cloned()
}

/// Run a recipe. It is checked against the repository as it is now, a
/// restore point is written, and its steps begin on a thread of their own.
#[tauri::command(async)]
pub fn recipe_run_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    recipes: State<'_, RecipesState>,
    recipe: Recipe,
) -> Result<RunView> {
    let repo = state
        .open_path("recipe_run_start")
        .map_err(|e| e.to_string())?;
    if let Some(live) = live_for(&recipes, &repo) {
        let over = live
            .run
            .try_lock()
            .map(|run| run.is_over())
            .unwrap_or(false);
        if !over {
            return Err("A recipe is already running here.".into());
        }
    }
    let prune = crate::settings::load(&app).prune_on_fetch;
    let world = world_for(&app, &repo, &recipe)?;

    /// Before there is a run there is nothing to tell a screen about.
    struct Quiet;
    impl Host for Quiet {
        fn say(&mut self, _line: &Line) {}
        fn check(&mut self, _repo: &Path, _command: &str) -> Checked {
            Checked {
                passed: false,
                output: String::new(),
            }
        }
        fn message(
            &mut self,
            _repo: &Path,
            _wanted: &Wanted,
        ) -> std::result::Result<String, String> {
            Err(String::new())
        }
        fn explain(&mut self, _repo: &Path, _failure: &Failure) -> Option<String> {
            None
        }
    }
    let run = Run::begin(&repo, &recipe, &world, prune, &mut Quiet)?;

    let (wake, woken) = channel();
    let live = Arc::new(Live {
        repo: repo.clone(),
        run: Mutex::new(run),
        wake: Mutex::new(wake),
        stop: Arc::new(AtomicBool::new(false)),
        working: AtomicBool::new(false),
        last: Mutex::new(None),
    });
    recipes
        .live
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(key_of(&repo), live.clone());
    let first = publish(&app, &live);
    let on_thread = live.clone();
    std::thread::Builder::new()
        .name("recipe-run".into())
        .spawn(move || drive(app, on_thread, woken))
        .map_err(|e| e.to_string())?;
    Ok(first)
}

/// The person's choice at a stop. An error means it cannot be taken yet —
/// a file is still conflicted — and the run goes on waiting.
#[tauri::command(async)]
pub fn recipe_run_answer<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    recipes: State<'_, RecipesState>,
    choice: Choice,
) -> Result<RunView> {
    let repo = state
        .open_path("recipe_run_answer")
        .map_err(|e| e.to_string())?;
    let live = live_for(&recipes, &repo).ok_or_else(|| "No recipe is running here.".to_string())?;
    {
        let mut run = live.run.lock().unwrap_or_else(|p| p.into_inner());
        let mut host = host_for(&app, &live, &run);
        run.answer(&repo, choice, &mut host)?;
    }
    let seen = publish(&app, &live);
    let _ = live.wake.lock().unwrap_or_else(|p| p.into_inner()).send(());
    Ok(seen)
}

/// *Stop*: the run ends before its next step. A step that is running is
/// let finish; a check that is running is ended.
#[tauri::command(async)]
pub fn recipe_run_stop<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    recipes: State<'_, RecipesState>,
) -> Result<()> {
    let repo = state
        .open_path("recipe_run_stop")
        .map_err(|e| e.to_string())?;
    let Some(live) = live_for(&recipes, &repo) else {
        return Ok(());
    };
    live.stop.store(true, Ordering::SeqCst);
    // Waiting at a stop, it is ended there. In a step, the flag is seen
    // before the next one.
    if let Ok(mut run) = live.run.try_lock() {
        if run
            .stop
            .as_ref()
            .is_some_and(|stop| stop.choices.contains(&Choice::End))
        {
            let mut host = host_for(&app, &live, &run);
            let _ = run.answer(&repo, Choice::End, &mut host);
        }
    }
    publish(&app, &live);
    let _ = live.wake.lock().unwrap_or_else(|p| p.into_inner()).send(());
    Ok(())
}

/// *Undo the whole run*: everything goes back to the restore point.
#[tauri::command(async)]
pub fn recipe_run_undo<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    recipes: State<'_, RecipesState>,
) -> Result<RunView> {
    let repo = state
        .open_path("recipe_run_undo")
        .map_err(|e| e.to_string())?;
    if let Some(live) = live_for(&recipes, &repo) {
        {
            let mut run = live
                .run
                .try_lock()
                .map_err(|_| "A step is running. Stop it first.".to_string())?;
            let mut host = host_for(&app, &live, &run);
            run.undo(&repo, &mut host)?;
        }
        let seen = publish(&app, &live);
        let _ = live.wake.lock().unwrap_or_else(|p| p.into_inner()).send(());
        return Ok(seen);
    }
    // A run from before Spagitty was closed: its restore point is still there.
    let folder = folder(&app, &repo)?;
    let mut run = last_record(&folder).ok_or_else(|| "There is no run to undo.".to_string())?;
    restore::undo(&repo, &run.restore).map_err(|e| e.to_string())?;
    run.stop = None;
    run.outcome = Some(run::Outcome::Undone);
    keep_record(&folder, &run)?;
    Ok(RunView {
        repo: key_of(&repo),
        run,
        live: false,
        working: false,
    })
}

/// The run on this repository: the one on a thread, or the last one's record.
#[tauri::command(async)]
pub fn recipe_run_current<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    recipes: State<'_, RecipesState>,
) -> Result<Option<RunView>> {
    let repo = state
        .open_path("recipe_run_current")
        .map_err(|e| e.to_string())?;
    if let Some(live) = live_for(&recipes, &repo) {
        // A step that is running holds the run. Whoever asks meanwhile is
        // told what the screen was last sent, which says a step is running.
        if let Ok(run) = live.run.try_lock() {
            return Ok(Some(view(&live, &run)));
        }
        return Ok(live.last.lock().unwrap_or_else(|p| p.into_inner()).clone());
    }
    Ok(last_record(&folder(&app, &repo)?).map(|run| RunView {
        repo: key_of(&repo),
        run,
        live: false,
        working: false,
    }))
}

/// Put a run that is over away, so the builder is shown again.
#[tauri::command(async)]
pub fn recipe_run_close(
    state: State<'_, AppState>,
    recipes: State<'_, RecipesState>,
) -> Result<()> {
    let repo = state
        .open_path("recipe_run_close")
        .map_err(|e| e.to_string())?;
    let mut live = recipes.live.lock().unwrap_or_else(|p| p.into_inner());
    let over = live.get(&key_of(&repo)).map(|live| {
        live.run
            .try_lock()
            .map(|run| run.is_over())
            .unwrap_or(false)
    });
    match over {
        Some(true) | None => {
            live.remove(&key_of(&repo));
            Ok(())
        }
        Some(false) => Err("The run is not over. Stop it first.".into()),
    }
}

// ── Restore points ───────────────────────────────────────────────────────

#[tauri::command(async)]
pub fn restore_points(state: State<'_, AppState>) -> Result<Vec<RestorePoint>> {
    let repo = state
        .open_path("restore_points")
        .map_err(|e| e.to_string())?;
    restore::list(&repo).map_err(|e| e.to_string())
}

/// Go back to a restore point. Refused while a recipe is running.
#[tauri::command(async)]
pub fn restore_point_undo(
    state: State<'_, AppState>,
    recipes: State<'_, RecipesState>,
    id: String,
) -> Result<()> {
    let repo = state
        .open_path("restore_point_undo")
        .map_err(|e| e.to_string())?;
    if let Some(live) = live_for(&recipes, &repo) {
        let over = live
            .run
            .try_lock()
            .map(|run| run.is_over())
            .unwrap_or(false);
        if !over {
            return Err("A recipe is running here. Undo it from Recipes.".into());
        }
    }
    restore::undo(&repo, &id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn restore_point_drop(state: State<'_, AppState>, id: String) -> Result<Vec<RestorePoint>> {
    let repo = state
        .open_path("restore_point_drop")
        .map_err(|e| e.to_string())?;
    restore::drop_point(&repo, &id).map_err(|e| e.to_string())?;
    restore::list(&repo).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spagitty_core::recipes::Message;

    fn named(name: &str) -> Recipe {
        Recipe {
            steps: vec![Step::Ask {}],
            ..Recipe::named(name)
        }
    }

    #[test]
    fn recipes_are_kept_in_a_file_of_their_own_and_read_back() {
        let dir = tempfile::tempdir().expect("a directory");
        let folder = dir.path().join("recipes").join("abc");

        // Not there yet is none, not an error.
        assert_eq!(read_recipes(&folder).expect("read"), Vec::<Recipe>::new());

        let kept = vec![named("sync-and-ship"), named("squash-for-pr")];
        write_recipes(&folder, &kept).expect("written");

        assert_eq!(read_recipes(&folder).expect("read"), kept);
        assert!(folder.join("recipes.json").is_file());
        assert!(!folder.join("recipes.json.partial").exists());
    }

    /// A file that cannot be read is left alone: an empty list written over
    /// it would drop every recipe in it.
    #[test]
    fn a_recipes_file_that_cannot_be_read_is_an_error_and_not_an_empty_list() {
        let dir = tempfile::tempdir().expect("a directory");
        std::fs::write(dir.path().join("recipes.json"), b"{ not json").expect("written");

        let refused = read_recipes(dir.path()).expect_err("refused");

        assert!(refused.contains("left as it is"));
    }

    #[test]
    fn a_recipe_is_saved_in_its_own_place_and_a_name_is_one_recipes() {
        let mut recipes = vec![named("one"), named("two")];

        // A new name goes to the end.
        put(&mut recipes, named("  three "), None).expect("added");
        assert_eq!(
            recipes.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["one", "two", "three"]
        );

        // Saved again, it replaces itself.
        let mut changed = named("two");
        changed.note = "changed".into();
        put(&mut recipes, changed, None).expect("replaced");
        assert_eq!(recipes.len(), 3);
        assert_eq!(recipes[1].note, "changed");

        // Renamed, it keeps its place.
        put(&mut recipes, named("uno"), Some("one")).expect("renamed");
        assert_eq!(recipes[0].name, "uno");
        assert_eq!(recipes.len(), 3);

        // Another recipe's name is refused, and nothing changes.
        let refused = put(&mut recipes, named("two"), Some("uno")).expect_err("taken");
        assert_eq!(refused, "There is already a recipe called two.");
        assert_eq!(recipes[0].name, "uno");
        assert_eq!(
            put(&mut recipes, named("   "), None).expect_err("unnamed"),
            "Name the recipe."
        );
    }

    #[test]
    fn a_recipe_is_imported_from_the_json_it_was_exported_as() {
        let exported = serde_json::to_string(&Recipe {
            steps: vec![Step::Commit {
                all: true,
                message: Message::text("Sync with main"),
            }],
            ..Recipe::named("sync")
        })
        .expect("exported");

        assert_eq!(import(&exported).expect("imported").name, "sync");
        assert!(import("not json")
            .expect_err("refused")
            .starts_with("That is not a recipe"));
        // A block that is not in the catalogue is not imported.
        let forced = r#"{"v":1,"name":"x","steps":[{"block":"forcePush","params":{}}]}"#;
        assert!(import(forced).is_err());
        let newer = r#"{"v":2,"name":"x","steps":[]}"#;
        assert_eq!(
            import(newer).expect_err("newer"),
            "That recipe is version 2. This Spagitty reads version 1."
        );
    }

    #[test]
    fn the_branches_that_may_not_be_pushed_to_unattended_include_the_ones_a_recipe_makes() {
        let branches = vec!["main".to_string(), "topic".to_string()];
        let recipe = Recipe {
            steps: vec![Step::NewBranch {
                name: "release/3.0".into(),
                from: String::new(),
            }],
            ..Recipe::named("cut")
        };
        let never = |name: &str| !(name == "main" || name.starts_with("release/"));

        assert_eq!(
            protected(&branches, &recipe, never),
            ["main", "release/3.0"]
        );
        assert_eq!(
            protected(&branches, &named("r"), |_| true),
            Vec::<String>::new()
        );
    }

    #[test]
    fn the_levels_are_the_same_four_under_the_same_names() {
        for (farm, recipe) in [
            (FarmLevel::Suggest, Level::Suggest),
            (FarmLevel::StepByStep, Level::StepByStep),
            (FarmLevel::SignOff, Level::SignOff),
            (FarmLevel::Unattended, Level::Unattended),
        ] {
            assert_eq!(level_of(farm), recipe);
            assert_eq!(
                serde_json::to_string(&farm).expect("json"),
                serde_json::to_string(&recipe).expect("json")
            );
        }
    }

    /// A stand-in for a run, to keep and read back.
    fn a_run(id: &str) -> Run {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "recipe": named("r"),
            "level": "stepByStep",
            "branch": "main",
            "started": 1,
            "ended": null,
            "steps": [],
            "at": 0,
            "stop": null,
            "outcome": null,
            "restore": id,
            "lines": [],
            "answers": [],
            "prune": false
        }))
        .expect("a run")
    }

    #[test]
    fn a_runs_record_is_kept_and_the_newest_is_the_one_read_back() {
        let dir = tempfile::tempdir().expect("a directory");

        assert!(last_record(dir.path()).is_none());
        keep_record(dir.path(), &a_run("1800000000-aaaa")).expect("kept");
        keep_record(dir.path(), &a_run("1800000100-bbbb")).expect("kept");
        keep_record(dir.path(), &a_run("1800000050-cccc")).expect("kept");

        assert_eq!(last_record(dir.path()).expect("read").id, "1800000100-bbbb");
    }

    #[test]
    fn only_the_newest_twenty_records_are_kept() {
        let dir = tempfile::tempdir().expect("a directory");

        for n in 0..25 {
            keep_record(dir.path(), &a_run(&format!("18000000{n:02}-aaaa"))).expect("kept");
        }

        let left = records(dir.path());
        assert_eq!(left.len(), RECORDS);
        assert!(left[0].ends_with("1800000024-aaaa.json"));
        assert!(left[19].ends_with("1800000005-aaaa.json"));
    }

    // ── Through the commands, against a real repository ──────────────────

    use crate::testing::{self, Emitted};
    use spagitty_core::fixture::Fixture;
    use spagitty_core::recipes::run::{Outcome, StopKind};
    use spagitty_core::recipes::MergeHow;
    use tauri::test::MockRuntime;
    use tauri::App;

    /// `main` with a commit of its own, and `other` with one that does not
    /// conflict with it.
    fn two_branches() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "one\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("First");
        fixture.git(&["switch", "-q", "-c", "other"]);
        fixture.write("other.txt", "from other\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Other's work");
        fixture.git(&["switch", "-q", "main"]);
        fixture
    }

    /// An application with `fixture` open in it.
    fn app_on(fixture: &Fixture) -> App<MockRuntime> {
        let app = testing::app();
        app.manage(RecipesState::default());
        crate::commands::open_repo(
            app.handle().clone(),
            app.state::<AppState>(),
            fixture.path().to_path_buf(),
            None,
        )
        .expect("the repository opened");
        app
    }

    fn merge_other(level: Level) -> Recipe {
        Recipe {
            level,
            steps: vec![
                Step::Merge {
                    branch: "other".into(),
                    how: MergeHow::Commit,
                },
                Step::Clean {},
            ],
            ..Recipe::named("bring-other-in")
        }
    }

    fn current(app: &App<MockRuntime>) -> Option<RunView> {
        recipe_run_current(
            app.handle().clone(),
            app.state::<AppState>(),
            app.state::<RecipesState>(),
        )
        .expect("the run")
    }

    fn start(app: &App<MockRuntime>, recipe: Recipe) -> Result<RunView> {
        recipe_run_start(
            app.handle().clone(),
            app.state::<AppState>(),
            app.state::<RecipesState>(),
            recipe,
        )
    }

    fn answer(app: &App<MockRuntime>, choice: Choice) -> Result<RunView> {
        recipe_run_answer(
            app.handle().clone(),
            app.state::<AppState>(),
            app.state::<RecipesState>(),
            choice,
        )
    }

    fn waits(app: &App<MockRuntime>) -> RunView {
        assert!(
            testing::wait_for(|| current(app).is_some_and(|view| view.run.stop.is_some())),
            "the run never stopped to ask"
        );
        current(app).expect("the run")
    }

    fn ends(app: &App<MockRuntime>) -> RunView {
        assert!(
            testing::wait_for(|| current(app).is_some_and(|view| view.run.is_over())),
            "the run never ended"
        );
        current(app).expect("the run")
    }

    #[test]
    fn a_recipe_is_saved_listed_and_checked_against_the_open_repository() {
        let fixture = two_branches();
        let app = app_on(&fixture);
        let recipe = merge_other(Level::StepByStep);

        let saved = recipes_save(
            app.handle().clone(),
            app.state::<AppState>(),
            recipe.clone(),
            None,
        )
        .expect("saved");
        assert_eq!(saved, std::slice::from_ref(&recipe));
        let listed = recipes_list(app.handle().clone(), app.state::<AppState>()).expect("listed");
        assert_eq!(listed.recipes, std::slice::from_ref(&recipe));
        assert_eq!(listed.guide_version, 1);

        let check = recipes_check(
            app.handle().clone(),
            app.state::<AppState>(),
            recipe.clone(),
        )
        .expect("checked");
        assert_eq!(check.findings, []);
        assert_eq!(check.world.branches, ["main", "other"]);
        assert_eq!(check.world.current.as_deref(), Some("main"));
        assert_eq!(
            check.commands,
            [
                "git merge --no-ff --no-edit other",
                "git status --porcelain"
            ]
        );
        assert_eq!(
            check.labels[0],
            "Merge other into main — always a merge commit"
        );
        // With the help off there is no brief, and there is always a script.
        assert_eq!(check.brief, None);
        assert!(check.script.contains("git merge --no-ff --no-edit other\n"));
        // A repository nobody has set rules for stops before the last act.
        assert_eq!(check.world.highest, Level::SignOff);
        assert_eq!(check.world.protected, ["main"]);

        // What is wrong with a stack is found against this repository.
        let mut wrong = recipe;
        wrong.steps[0] = Step::Merge {
            branch: "nowhere".into(),
            how: MergeHow::Commit,
        };
        let found =
            recipes_check(app.handle().clone(), app.state::<AppState>(), wrong).expect("checked");
        assert_eq!(found.errors, 1);
        assert_eq!(found.findings[0].code, "branch.missing");

        let left = recipes_delete(
            app.handle().clone(),
            app.state::<AppState>(),
            "bring-other-in".into(),
        )
        .expect("deleted");
        assert!(left.is_empty());
    }

    #[test]
    fn with_the_help_on_the_brief_is_the_guide_and_then_the_recipe() {
        let fixture = two_branches();
        let app = app_on(&fixture);
        let recipe = Recipe {
            agent_help: true,
            note: "Keep it short.".into(),
            ..merge_other(Level::StepByStep)
        };

        let check =
            recipes_check(app.handle().clone(), app.state::<AppState>(), recipe).expect("checked");

        let brief = check.brief.expect("a brief");
        assert!(brief.starts_with("# Spagitty recipes"));
        assert!(brief.contains("recipe: bring-other-in\n"));
        assert!(brief.contains("branch: main\n"));
        assert!(brief.contains("agent: (no agent chosen) · level: step\n"));
        assert!(brief.contains("## Note from Ada Lovelace\nKeep it short.\n"));
    }

    /// A run on its thread: it runs through, says so, and keeps its record.
    #[test]
    fn a_run_runs_on_a_thread_of_its_own_and_tells_the_screen() {
        let fixture = two_branches();
        let app = app_on(&fixture);
        let runs: Emitted<serde_json::Value> = Emitted::on(app.handle(), EVENT);
        let lines: Emitted<serde_json::Value> = Emitted::on(app.handle(), LINE);

        // *Ask before pushing* asks before neither of these steps.
        let first = start(&app, merge_other(Level::SignOff)).expect("started");
        assert!(first.live);
        assert_eq!(first.run.steps.len(), 2);
        let done = ends(&app);

        assert_eq!(done.run.outcome, Some(Outcome::Done));
        assert_eq!(
            fixture.git(&["log", "-1", "--format=%s"]).trim(),
            "Merge branch 'other'"
        );
        // The screen was sent the run, and each line as it was printed.
        let last = runs.at_least(2).pop().expect("the last event");
        assert_eq!(last["run"]["outcome"], "done");
        assert_eq!(last["repo"], fixture.path().to_string_lossy().as_ref());
        assert!(lines
            .all()
            .iter()
            .any(|event| event["line"]["text"] == "$ git merge --no-ff --no-edit other"));
        // The record is kept, and the restore point with it.
        let folder = folder(app.handle(), fixture.path()).expect("a folder");
        let record = last_record(&folder).expect("a record");
        assert_eq!(record.id, done.run.id);
        assert_eq!(record.outcome, Some(Outcome::Done));
        let points = restore_points(app.state::<AppState>()).expect("listed");
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].id, done.run.restore);

        // Over, it is put away.
        recipe_run_close(app.state::<AppState>(), app.state::<RecipesState>()).expect("closed");
        let after = current(&app).expect("the record");
        assert!(!after.live);
    }

    #[test]
    fn a_run_waits_for_the_person_and_goes_on_when_answered() {
        let fixture = two_branches();
        let app = app_on(&fixture);
        start(&app, merge_other(Level::StepByStep)).expect("started");

        let waiting = waits(&app);
        assert_eq!(
            waiting.run.stop.as_ref().map(|stop| stop.kind),
            Some(StopKind::Approve)
        );
        // Nothing has run, and nothing does while it waits.
        assert_eq!(fixture.git(&["log", "-1", "--format=%s"]).trim(), "First");
        // A second recipe is refused while this one is going.
        assert_eq!(
            start(&app, merge_other(Level::SignOff)).expect_err("refused"),
            "A recipe is already running here."
        );
        // And it cannot be put away.
        assert!(recipe_run_close(app.state::<AppState>(), app.state::<RecipesState>()).is_err());

        answer(&app, Choice::Continue).expect("go on");
        let again = waits(&app);
        assert_eq!(again.run.at, 1);
        assert_eq!(
            fixture.git(&["log", "-1", "--format=%s"]).trim(),
            "Merge branch 'other'"
        );
        answer(&app, Choice::Continue).expect("go on");

        assert_eq!(ends(&app).run.outcome, Some(Outcome::Done));
    }

    /// A step holds the run for as long as it takes. Asking what the run is
    /// doing must not wait for the step.
    #[test]
    fn asking_about_a_run_does_not_wait_for_the_step_that_is_running() {
        let fixture = two_branches();
        let app = app_on(&fixture);
        start(&app, merge_other(Level::StepByStep)).expect("started");
        let waiting = waits(&app);
        let live = live_for(&app.state::<RecipesState>(), fixture.path()).expect("a run");

        // The run is held, as a step holds it.
        let held = live.run.lock().expect("the run");
        let (answered, answer) = std::sync::mpsc::channel();
        let handle = app.handle().clone();
        std::thread::spawn(move || {
            let seen = recipe_run_current(
                handle.clone(),
                handle.state::<AppState>(),
                handle.state::<RecipesState>(),
            );
            let _ = answered.send(seen);
        });
        let seen = answer
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("an answer while the run was held")
            .expect("the run")
            .expect("what was last sent");
        drop(held);

        assert_eq!(seen.run.id, waiting.run.id);
        assert!(seen.live);
        assert_eq!(
            seen.run.stop.as_ref().map(|stop| stop.kind),
            Some(StopKind::Approve)
        );
    }

    #[test]
    fn a_choice_that_cannot_be_taken_leaves_the_run_waiting() {
        let fixture = two_branches();
        let app = app_on(&fixture);
        start(&app, merge_other(Level::StepByStep)).expect("started");
        waits(&app);

        let refused = answer(&app, Choice::Skip).expect_err("not offered");

        assert_eq!(refused, "That is not one of the choices here.");
        assert!(current(&app).is_some_and(|view| view.run.stop.is_some()));
    }

    #[test]
    fn stop_ends_a_run_that_is_waiting() {
        let fixture = two_branches();
        let app = app_on(&fixture);
        start(&app, merge_other(Level::StepByStep)).expect("started");
        waits(&app);

        recipe_run_stop(
            app.handle().clone(),
            app.state::<AppState>(),
            app.state::<RecipesState>(),
        )
        .expect("stopped");

        let ended = ends(&app);
        assert_eq!(ended.run.outcome, Some(Outcome::Ended));
        assert_eq!(fixture.git(&["log", "-1", "--format=%s"]).trim(), "First");
    }

    #[test]
    fn undoing_the_whole_run_puts_the_repository_back() {
        let fixture = two_branches();
        fixture.write("notes.txt", "mine, untracked\n");
        let before = [fixture.head(), fixture.git(&["status", "--porcelain"])];
        let app = app_on(&fixture);
        start(&app, merge_other(Level::SignOff)).expect("started");
        // The second step stops: the tree is not clean.
        let stopped = waits(&app);
        assert_eq!(
            stopped.run.stop.as_ref().map(|stop| stop.kind),
            Some(StopKind::NotClean)
        );
        assert_ne!(fixture.head(), before[0]);

        let undone = recipe_run_undo(
            app.handle().clone(),
            app.state::<AppState>(),
            app.state::<RecipesState>(),
        )
        .expect("undone");

        assert_eq!(undone.run.outcome, Some(Outcome::Undone));
        assert_eq!(
            [fixture.head(), fixture.git(&["status", "--porcelain"])],
            before
        );
        assert_eq!(
            std::fs::read_to_string(fixture.at("notes.txt")).expect("the file"),
            "mine, untracked\n"
        );
        ends(&app);
    }

    #[test]
    fn a_restore_point_is_gone_back_to_and_dropped_from_the_list() {
        let fixture = two_branches();
        let before = fixture.head();
        let app = app_on(&fixture);
        start(&app, merge_other(Level::SignOff)).expect("started");
        let done = ends(&app);
        assert_ne!(fixture.head(), before);

        restore_point_undo(
            app.state::<AppState>(),
            app.state::<RecipesState>(),
            done.run.restore.clone(),
        )
        .expect("back");
        assert_eq!(fixture.head(), before);

        let left = restore_point_drop(app.state::<AppState>(), done.run.restore).expect("dropped");
        assert!(left.is_empty());
    }

    #[test]
    fn a_stack_that_cannot_run_does_not_start() {
        let fixture = two_branches();
        let app = app_on(&fixture);

        let refused = start(&app, merge_other(Level::Suggest)).expect_err("read-only");
        assert!(refused.starts_with("Read-only runs nothing."));
        assert!(current(&app).is_none());

        // With the help off nobody proposes anything either.
        let nobody = recipes_propose(
            app.handle().clone(),
            app.state::<AppState>(),
            merge_other(Level::StepByStep),
            "Bring other in".into(),
        )
        .expect_err("off");
        assert_eq!(nobody, "Agent help is off.");
    }

    #[test]
    fn with_no_repository_open_every_command_says_so() {
        let app = testing::app();
        app.manage(RecipesState::default());

        assert!(recipes_list(app.handle().clone(), app.state::<AppState>()).is_err());
        assert!(restore_points(app.state::<AppState>()).is_err());
        assert!(start(&app, merge_other(Level::SignOff)).is_err());
    }
}
