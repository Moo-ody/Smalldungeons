use crate::server::entity::ai::ai_enum::TaskType;
use crate::server::entity::ai::task_data::TaskData;
use crate::server::world::World;
use indexmap::IndexSet;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Representation of minecraft's [EntityAiTasks](https://github.com/Marcelektro/MCP-919/blob/main/src/minecraft/net/minecraft/entity/ai/EntityAITasks.java).
///
/// basic task data is stored in the tasks indexset taskentry, such as priority
/// executing, and data use the tasktype as the key, however this is not strictly enforced by taskentry, so be careful.
///
/// insertion matter matters for task execution, but the other sets dont need it.
#[derive(Clone, Debug)]
pub struct AiTasks {
    tasks: IndexSet<TaskEntry>, // executing and data tasktypes could be changed to pointers with explicit lifetimes since they could all be the same value as the taskentry type but idk if i want to bother with that rn.
    executing: HashSet<TaskType>,
    data: HashMap<TaskType, TaskData>,

    tick_count: i32,
    tick_rate: i32,
}

impl AiTasks {
    pub fn new(/*tick_rate: i32*/) -> Self {
        Self {
            tasks: IndexSet::new(),
            executing: HashSet::new(),
            data: HashMap::new(),

            tick_count: 0,
            tick_rate: 3,
        }
    }

    pub fn create_from_entries(task_entries: Vec<TaskEntry>) -> Self {
        let mut tasks = Self::new();
        for TaskEntry { priority, task_type } in task_entries {
            tasks.add_task(priority, task_type);
        }
        tasks
    }

    pub fn add_task(&mut self, priority: u8, task_type: TaskType) {
        self.tasks.insert(TaskEntry { priority, task_type });
        self.data.insert(task_type, TaskData::default(task_type));
    }

    fn update(&mut self, executing_entity: &i32, world: &mut World) -> anyhow::Result<()> {
        self.tick_count += 1;
        if self.tick_count % self.tick_rate == 0 {
            for task in self.tasks.iter() {
                let data = self.data.get_mut(&task.task_type).ok_or_else(|| anyhow::anyhow!("Task data for {task:?} not found."))?;
                if self.executing.contains(&task.task_type) {
                    if Self::can_use(&self.tasks, &self.executing, task, data) && data.keep_executing(executing_entity, world) {
                        continue;
                    }
                    data.reset();
                    self.executing.remove(&task.task_type);
                } else {
                    if !Self::can_use(&self.tasks, &self.executing, task, data) || !data.should_run(executing_entity, world) {
                        continue;
                    }
                    data.start_executing(executing_entity, world);
                    self.executing.insert(task.task_type);
                }
            }
        } else {
            self.executing.retain(|task| {
                let data = self.data.get_mut(task).expect("Task data for executing task not found. This should be impossible!"); // this consumes our return operator so either we print this or just panic. This should be impossible to fail though
                if data.should_continue(executing_entity, world) { return true; }
                data.reset();
                false
            });
        }

        for task_type in self.executing.iter() {
            let data = self.data.get_mut(task_type).ok_or(anyhow::anyhow!("Task data for executing task: {task_type:?} not found."))?;
            data.update(executing_entity, world);
        }

        Ok(())
    }

    // this cant use &self because self.data is borrowed mutably when this is called.
    fn can_use(tasks: &IndexSet<TaskEntry>, executing_tasks: &HashSet<TaskType>, task: &TaskEntry, data: &mut TaskData) -> bool {
        todo!();
        for task_entry in tasks {
            if task_entry == task || !(task_entry.priority >= task.priority) {}
        }
        return true;
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskEntry {
    priority: u8,
    task_type: TaskType,
}

impl TaskEntry {
    pub fn new(priority: u8, task_type: TaskType) -> Self {
        Self { priority, task_type }
    }
}