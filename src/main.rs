use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{self, BufReader, BufWriter};

#[derive(Debug, Clone)]
enum NodeType {
    FILE,
    DIR { children: HashMap<String, usize> },
}

/*
enum AppError {
    InvalidPath,
    InvalidDirectory,
    InvalidFile,
    BackupFileError,
    BackupParseError,
}
*/
#[derive(Debug, Clone)]
struct FsNode {
    name: String,
    parent: usize,
    node_type: NodeType,
}

impl FsNode {
    fn new_file_node(name: &str, parent: usize) -> FsNode {
        FsNode {
            name: name.to_string(),
            parent,
            node_type: NodeType::FILE,
        }
    }
    fn new_dir_node(name: &str, parent: usize) -> FsNode {
        FsNode {
            name: name.to_string(),
            parent,
            node_type: NodeType::DIR {
                children: HashMap::new(),
            },
        }
    }

    fn is_file_node(&self) -> bool {
        if let NodeType::FILE = self.node_type {
            true
        } else {
            false
        }
    }

    fn is_dir_node(&self) -> bool {
        !self.is_file_node()
    }
}

struct FileSystem {
    counter: usize,
    cwd: usize,
    nodes: HashMap<usize, FsNode>,
}

type FsResult = Result<(), &'static str>;

impl FileSystem {
    fn new() -> FileSystem {
        let counter = 0;
        let root = FsNode::new_dir_node("/", counter);
        let mut nodes = HashMap::new();
        nodes.insert(counter, root);
        FileSystem {
            counter,
            cwd: counter,
            nodes,
        }
    }

    // finds the node represented by the path
    fn find(&self, start_id: usize, path: &[&str]) -> Result<usize, &'static str> {
        let mut iter = path.iter().peekable();
        let mut current_id = start_id;
        while let Some(name) = iter.next() {
            // find the current name among the current node siblings
            let current_node = self.nodes.get(&current_id).unwrap();
            if let NodeType::DIR { children } = &current_node.node_type {
                current_id = *children
                    .get(*name)
                    .ok_or_else(|| "No such file or directory")?;
            } else if iter.peek().is_some() {
                return Err("Not a directory");
            }
        }
        Ok(current_id)
    }

    fn mkdir(&mut self, path_name: &str) -> FsResult {
        let path = split_path(path_name);
        if let Some((dir_name, base_path)) = path.split_last() {
            let start_id = if path_name.starts_with('/') {
                0
            } else {
                self.cwd
            };
            let target_id = self.find(start_id, base_path)?;
            let target_node = self.nodes.get_mut(&target_id).unwrap();
            match &mut target_node.node_type {
                NodeType::FILE => Err("Not a directory"),
                NodeType::DIR { children } => {
                    if children.contains_key(*dir_name) {
                        return Err("Directory already exists");
                    };
                    let new_node = FsNode::new_dir_node(*dir_name, target_id);
                    let new_counter = self.counter + 1;
                    children.insert((*dir_name).to_string(), new_counter);
                    self.nodes.insert(new_counter, new_node);
                    self.counter = new_counter;
                    Ok(())
                }
            }
        } else {
            Err("missing path")
        }
    }

    fn pwd(&self) {
        let mut node = self.nodes.get(&self.cwd).unwrap();
        let mut cwd_vec: Vec<&str> = Vec::new();
        if &node.name != "/" {
            cwd_vec.push(&node.name);
        }
        while node.parent != 0 {
            node = self.nodes.get(&node.parent).unwrap();
            cwd_vec.push(&node.name);
        }
        cwd_vec.reverse();

        println!("/{}", cwd_vec.join("/"));
    }

    fn ls(&self, path: Option<String>) -> FsResult {
        let fsnode = if let Some(path) = path {
            let start_id = if path.starts_with('/') { 0 } else { self.cwd };
            let path = split_path(&path);
            let target_id = self.find(start_id, &path)?;
            self.nodes.get(&target_id).unwrap()
        } else {
            self.nodes.get(&self.cwd).unwrap()
        };
        match &fsnode.node_type {
            NodeType::DIR { children } => children
                .keys()
                .for_each(|child_name| println!("{}", child_name)),
            NodeType::FILE => return Err("not a directory"),
        }
        Ok(())
    }

    fn cd(&mut self, path: Option<String>) -> FsResult {
        if let Some(path) = path {
            let start_id = if path.starts_with('/') { 0 } else { self.cwd };
            let path = split_path(&path);
            let target_id = self.find(start_id, &path)?;
            let node = self.nodes.get(&target_id).unwrap();
            match &node.node_type {
                NodeType::DIR { children: _ } => self.cwd = target_id,
                NodeType::FILE => return Err("not a directory"),
            }
        } else {
            self.cwd = 0;
        };
        Ok(())
    }

    /*
    fn get_children(&self, parent_id: usize) -> Result<&HashMap<String, usize>, &'static str> {
        let parent_node = self.nodes.get(&parent_id).unwrap();
        match &parent_node.node_type {
            NodeType::FILE => Err("not a directory"),
            NodeType::DIR { children } => Ok(children),
        }
    }
    */

    fn rmdir(&mut self, path_name: &str) -> FsResult {
        let path = split_path(path_name);
        let start_id = if path_name.starts_with('/') {
            0
        } else {
            self.cwd
        };
        let target_id = self.find(start_id, &path)?;
        let target_node = self.nodes.get(&target_id).unwrap();
        if let NodeType::DIR { children } = &target_node.node_type {
            if !children.is_empty() {
                return Err("Directory not empty");
            }
        } else {
            return Err("not a directory");
        }

        let parent_id: usize = (&target_node.parent).to_owned();
        let target_name = (&target_node.name).clone();
        if let NodeType::DIR { children } = &mut self.nodes.get_mut(&parent_id).unwrap().node_type {
            children.remove(&target_name);
        };

        Ok(())
    }

    fn creat(&mut self, path_name: &str) -> FsResult {
        let path = split_path(path_name);
        if let Some((file_name, base_path)) = path.split_last() {
            let start_id = if path_name.starts_with('/') {
                0
            } else {
                self.cwd
            };
            let target_id = self.find(start_id, base_path)?;
            let target_node = self.nodes.get_mut(&target_id).unwrap();
            match &mut target_node.node_type {
                NodeType::FILE => Err("Not a directory"),
                NodeType::DIR { children } => {
                    if children.contains_key(*file_name) {
                        return Err("File already exists");
                    };
                    let new_node = FsNode::new_file_node(*file_name, target_id);
                    let new_counter = self.counter + 1;
                    children.insert((*file_name).to_string(), new_counter);
                    self.nodes.insert(new_counter, new_node);
                    self.counter = new_counter;
                    Ok(())
                }
            }
        } else {
            Err("missing path")
        }
    }

    fn rm(&mut self, path_name: &str) -> FsResult {
        let path = split_path(path_name);
        let start_id = if path_name.starts_with('/') {
            0
        } else {
            self.cwd
        };
        let target_id = self.find(start_id, &path)?;
        let target_node = self.nodes.get(&target_id).unwrap();

        if target_node.is_dir_node() {
            return Err("not a file");
        }

        let parent_id: usize = (&target_node.parent).to_owned();
        let target_name = (&target_node.name).clone();
        if let NodeType::DIR { children } = &mut self.nodes.get_mut(&parent_id).unwrap().node_type {
            children.remove(&target_name);
        };

        Ok(())
    }

    fn save(&self, maybe_filepath: Option<String>) -> FsResult {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(maybe_filepath.unwrap_or_else(|| "backup.fs".to_string()))
            .map_err(|_| "Error opening the backup file")?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, "{} {}", self.counter, self.nodes.len())
            .map_err(|_| "error writing to backup file")?;
        // writeln!(writer, "{}", self.nodes.len()).map_err(|_| "error writing to backup file")?;
        for (id, node) in self.nodes.iter() {
            writeln!(writer, "{} {}", id, node.name).map_err(|_| "error writing to backup file")?;
        }
        for (id, node) in self.nodes.iter() {
            match &node.node_type {
                NodeType::DIR { children } => writeln!(
                    writer,
                    "D {} {} {}",
                    id,
                    node.parent,
                    children
                        .values()
                        .map(|idx| idx.to_string())
                        .collect::<Vec<String>>()
                        .join(",")
                ),
                NodeType::FILE => writeln!(writer, "F {} {}", id, node.parent.to_string()),
            }
            .map_err(|_| "Error writing to file")?;
        }
        Ok(())
    }

    fn reload(&mut self, maybe_filepath: Option<String>) -> FsResult {
        let file = File::open(maybe_filepath.unwrap_or_else(|| "backup.fs".to_string()))
            .map_err(|_| "Error opening the backup file")?;
        let mut reader = BufReader::new(&file);

        let mut buffer = String::new();
        reader
            .read_line(&mut buffer)
            .map_err(|_| "Error reading the backup file")?;
        let counter;
        let total_nodes;
        if let [counter_str, total_nodes_str] =
            buffer.trim().split(' ').collect::<Vec<&str>>().as_slice()
        {
            counter = counter_str
                .parse::<usize>()
                .map_err(|_| "Error parsing the backup: error reading counter")?;
            total_nodes = total_nodes_str
                .parse::<usize>()
                .map_err(|_| "Error parsing the backup: error reading total_nodes")?;
        } else {
            return Err("Error parsing the backup: not two number on first line");
        };
        let mut index = HashMap::new();
        for _ in 0..total_nodes {
            let mut buffer = String::new();
            reader
                .read_line(&mut buffer)
                .map_err(|_| "Error reading the backup file")?;
            if let [id_str, name] = buffer.trim().split(' ').collect::<Vec<&str>>().as_slice() {
                let id = id_str
                    .parse::<usize>()
                    .map_err(|_| "Error parsing the backup: not two numbers for index")?;
                index.insert(id, name.to_string());
            }
        }
        let mut nodes = HashMap::new();
        for _ in 0..total_nodes {
            let mut buffer = String::new();
            reader
                .read_line(&mut buffer)
                .map_err(|_| "Error reading the backup file")?;

            if let ["D", id_str, parent_id_str, children_str] =
                buffer.trim().split(' ').collect::<Vec<&str>>().as_slice()
            {
                let id = id_str
                    .parse::<usize>()
                    .map_err(|_| "Error parsing the backup: not two numbers for index")?;
                let name = index.get(&id).ok_or("Error rebuilding the backup")?.clone();
                let parent = parent_id_str
                    .parse::<usize>()
                    .map_err(|_| "Error parsing the backup: not two numbers for index")?;
                let children = children_str
                    .split(',')
                    .map(|s| s.parse::<usize>())
                    .map(|r| r.map_err(|_| "Error parsing the backup: not two numbers for index"))
                    .map(|r| {
                        r.and_then(|i| {
                            index
                                .get(&i)
                                .map(|v| (v.clone(), i))
                                .ok_or_else(|| "Error parsing")
                        })
                    })
                    .collect::<Result<HashMap<String, usize>, _>>()
                    .map_err(|_| "Error parsing the backup: not two numbers for index")?;

                let node = FsNode {
                    name,
                    parent,
                    node_type: NodeType::DIR { children },
                };
                nodes.insert(id, node);
            } else if let ["D", id_str, parent_id_str] =
                buffer.trim().split(' ').collect::<Vec<&str>>().as_slice()
            {
                let id = id_str
                    .parse::<usize>()
                    .map_err(|_| "Error parsing the backup: not two numbers for index")?;
                let name = index.get(&id).ok_or("Error rebuilding the backup")?.clone();
                let parent = parent_id_str
                    .parse::<usize>()
                    .map_err(|_| "Error parsing the backup: not two numbers for index")?;
                let node = FsNode {
                    name,
                    parent,
                    node_type: NodeType::DIR {
                        children: HashMap::new(),
                    },
                };
                nodes.insert(id, node);
            } else if let ["F", id_str, parent_id_str] =
                buffer.trim().split(' ').collect::<Vec<&str>>().as_slice()
            {
                let id = id_str
                    .parse::<usize>()
                    .map_err(|_| "Error parsing the backup: not two numbers for index")?;
                let name = index.get(&id).ok_or("Error rebuilding the backup")?.clone();
                let parent = parent_id_str
                    .parse::<usize>()
                    .map_err(|_| "Error parsing the backup: not two numbers for index")?;
                let node = FsNode {
                    name,
                    parent,
                    node_type: NodeType::FILE,
                };
                nodes.insert(id, node);
            } else {
                return Err("Error rebuilding the backup");
            }
        }

        self.nodes = nodes;
        self.cwd = 0;
        self.counter = counter;
        Ok(())
    }
}

fn split_path(path_name: &str) -> Vec<&str> {
    path_name
        .trim_matches('/')
        .split('/')
        .filter(|name| *name != "")
        .collect()
}

enum Command {
    Pwd,
    Quit,
    MkDir(String),
    Creat(String),
    RmDir(String),
    Rm(String),
    Ls(Option<String>),
    Cd(Option<String>),
    Save(Option<String>),
    Reload(Option<String>),
    NoOp,
}

fn parse_command(command: &str) -> Result<Command, &'static str> {
    let mut iter = command.trim().split(' ');
    match iter.next() {
        Some("pwd") => Ok(Command::Pwd),
        Some("quit") => Ok(Command::Quit),
        Some("mkdir") => {
            if let Some(filename) = iter.next() {
                Ok(Command::MkDir(filename.to_string()))
            } else {
                Err("missing operand")
            }
        }
        Some("ls") => Ok(Command::Ls(iter.next().map(|name| name.to_string()))),
        Some("cd") => Ok(Command::Cd(iter.next().map(|name| name.to_string()))),
        Some("rmdir") => iter
            .next()
            .ok_or("missing operand")
            .and_then(|path| {
                if path == "/" {
                    Err("cannot remove root directory")
                } else {
                    Ok(path)
                }
            })
            .map(|path| Command::RmDir(path.to_string())),
        Some("creat") => iter
            .next()
            .ok_or("missing operand")
            .map(|path| Command::Creat(path.to_string())),
        Some("rm") => iter
            .next()
            .ok_or("missing operand")
            .map(|path| Command::Rm(path.to_string())),
        Some("save") => Ok(Command::Save(iter.next().map(|name| name.to_string()))),
        Some("reload") => Ok(Command::Reload(iter.next().map(|name| name.to_string()))),
        Some("") => Ok(Command::NoOp),
        _ => Err("not implemented"),
    }
}

fn main() {
    let mut fs = FileSystem::new();
    loop {
        let mut command = String::new();
        print!("$ ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut command).unwrap();
        match parse_command(&command) {
            Ok(command) => match command {
                Command::Pwd => fs.pwd(),
                Command::Quit => {
                    println!("Saving...");
                    fs.save(None)
                        .unwrap_or_else(|err| println!("Quitting without saving: {}", err));
                    break;
                }
                Command::MkDir(filename) => fs.mkdir(&filename).unwrap_or_else(|err| {
                    println!("mkdir: cannot create directory {}: {}", filename, err)
                }),
                Command::Ls(filename) => fs.ls(filename).unwrap_or_else(|err| println!("{}", err)),
                Command::Cd(filename) => fs
                    .cd(filename)
                    .unwrap_or_else(|err| println!("cd: {}", err)),

                Command::RmDir(filename) => fs
                    .rmdir(&filename)
                    .unwrap_or_else(|err| println!("rmdir: {}", err)),
                Command::Creat(filename) => fs.creat(&filename).unwrap_or_else(|err| {
                    println!("creat: cannot create file {}: {}", filename, err)
                }),
                Command::Rm(filename) => fs
                    .rm(&filename)
                    .unwrap_or_else(|err| println!("rm: cannot remove {}: {}", filename, err)),
                Command::Save(maybe_filename) => fs
                    .save(maybe_filename)
                    .unwrap_or_else(|err| println!("error saving the filesystem: {}", err)),
                Command::Reload(maybe_filename) => fs
                    .reload(maybe_filename)
                    .unwrap_or_else(|err| println!("error reloading the filesystem: {}", err)),
                Command::NoOp => continue,
            },
            Err(err) => println!("{}", err),
        }
    }
}
