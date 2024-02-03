use core::time;
use std::{fmt::Debug, thread};

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::MAIN_WINDOW;

pub struct Task {
    statue: TaskStatue,
    // command_sender: Option<a>,
    f: Option<Box<dyn Fn() + Send + Sync>>,
}

impl Debug for Task {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        println!("Progress: {:#?}", self.statue);
        Ok(())
    }
}

impl Clone for Task {
    fn clone(&self) -> Self {
        Self {
            statue: self.statue.clone(),
            f: None,
        }
    }
}

impl Task {
    pub fn start(&self, tasks: Arc<Mutex<Vec<Task>>>, index: usize) {
        let tasks_lock = tasks.l
        let mut task = tasks.get_mut(index).unwrap();
        task.statue.statue = State::Active;
        (self.f.as_ref()).unwrap()();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskStatue {
    progress: Option<(u64, u64)>,
    progress_unit: Option<String>,
    statue: State,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum State {
    Paused,
    Active,
    Success,
    Failed,
    Canceled,
    Waiting,
}

fn task_can_start(task: &Task) -> bool {
    true
}

fn task_spawn(tasks: Arc<Mutex<Vec<Task>>>, index: usize) {
    thread::spawn(move || {
        let mut tasks_lock = tasks.lock().unwrap();
        let current_task = tasks_lock.get(index).unwrap();
        current_task.start(tasks, index);
    });
}

static TASKS: OnceCell<Arc<Mutex<Vec<Task>>>> = OnceCell::new();

pub fn init_task_manager() /* -> Result<()>  */
{
    let tasks = Arc::new(Mutex::new(Vec::new()));
    TASKS.set(Arc::clone(&tasks)).unwrap();
    thread::spawn(move || loop {
        thread::sleep(time::Duration::from_millis(200));

        let tasks_arc = Arc::clone(&tasks);
        let tasks = tasks_arc.lock().unwrap();
        for (index, task) in tasks.iter().enumerate() {
            if !task_can_start(&task) {
                continue;
            }
            task_spawn(Arc::clone(TASKS.get().unwrap()), index);
            // tasks.remove(index);
            // tasks.insert(index, task)
        }
        MAIN_WINDOW
            .get()
            .unwrap()
            .emit(
                "download_tasks",
                tasks.iter().map(|x| &x.statue).collect::<Vec<_>>(),
            )
            .unwrap();
        // println!("{:#?}", tasks.try_lock().unwrap())
    });
    // test
    test()
}

fn test() {
    let tasks = Arc::clone(TASKS.get().unwrap());
    let mut tasks = tasks.lock().unwrap();
    let f = Box::new(|| {
        for _ in 0..5 {
            thread::sleep(time::Duration::from_millis(1000));
            println!("~1");
        }
    });
    let f2 = Box::new(|| {
        for _ in 0..5 {
            thread::sleep(time::Duration::from_millis(1000));
            println!("!1");
        }
    });
    tasks.push(Task {
        statue: TaskStatue {
            progress: Some((0, 0)),
            progress_unit: Some("".to_string()),
            statue: State::Waiting,
        },
        f: Some(f),
    });
    tasks.push(Task {
        statue: TaskStatue {
            progress: Some((0, 0)),
            progress_unit: Some("".to_string()),
            statue: State::Waiting,
        },
        f: Some(f2),
    });
    println!("{:#?}", tasks)
}
