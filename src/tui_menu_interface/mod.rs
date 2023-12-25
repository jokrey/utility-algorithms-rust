use std::{error, io};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Display};
use std::io::Write;
use std::ops::Deref;
use std::str::FromStr;
use std::path::Path;
use crate::general::DisplayableTuple;

//from: https://stackoverflow.com/a/27582993/13241877
macro_rules! map(
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);

//this design suffers a bit from stringified types,
//   which isn't too bad because after all we are working with strings and there is at least always the option of validating the input
//   but still.. The reason lies in the input collector method in menu - where generics do not work.
//   Keeping the input and choice instances as the caller and querying them directly is not ideal either, because 
pub trait MenuItem {
    ///Name of the menu item as displayed in the parent menu
    fn get_display_string(&self) -> String;

    ///called when the menu item is selected by the user
    /// should for example print what the user can/has to do now
    /// -> returns whether the operation can be considered successful
    fn notify_selected(&self, nest_level: usize, parent_menu: Option<&dyn MenuItem>) -> bool;

    ///Note: values of duplicate names in the menu tree might only be collected once(i.e. only one is in the resulting hashmap).
    fn collect_inputs(&self) -> HashMap<String, String> ;
}

const INDENTATION: usize = 3;

pub struct Menu<'a> {
    is_root: bool,
    name: &'a str,
    pub items: Vec<&'a dyn MenuItem>
}
impl Menu<'_> {
    pub fn run_root<'a>(name: &'a str, items :Vec<&'a dyn MenuItem>) -> Menu<'a> {
        let root = Menu::root(name, items);
        root.run();
        root
    }
    pub fn root<'a>(name: &'a str, items :Vec<&'a dyn MenuItem>) -> Menu<'a> {
        Menu {
            is_root : true,
            name,
            items
        }
    }
    pub fn run(&self) {
        if !self.is_root {
            panic!("Only root can be run");
        } else {
            self.notify_selected(0, None);
        }
    }
    pub fn sub<'a>(name: &'a str, items :Vec<&'a dyn MenuItem>) -> Menu<'a> {
        Menu {
            is_root : false,
            name,
            items
        }
    }
}

impl MenuItem for Menu<'_> {
    fn get_display_string(&self) -> String {
        return format!("{} (Menu)", self.name);
    }

    fn notify_selected(&self, nest_level: usize, _parent_menu: Option<&dyn MenuItem>) -> bool {
        loop {
            let items_length = self.items.len();
            print_with(nest_level * INDENTATION,&format!("Menu({}):\n", self.get_display_string())).expect("Fatal print error");
            for (pos, item) in self.items.iter().enumerate() {
                print_with(nest_level * INDENTATION, &format!("-> {}: {}\n", pos+1, item.get_display_string())).expect("Fatal print error");
            }
            print_with(nest_level * INDENTATION, &format!("-> {}: {}\n", items_length + 1, if self.is_root { "Finish" } else { "Back" })).expect("Fatal print error");


            print_f_with(nest_level * INDENTATION, "Select Item(by number or name): ").expect("Fatal print error");
            let selection = read_line().expect("Fatal print error");
            let selected_item = match selection.parse::<usize>() {
                Ok(number) => {
                    if number > 0 && number <= items_length {
                        Some(self.items[number - 1])
                    } else if number == items_length +1 {
                        return true;
                    } else {
                        None
                    }
                }
                //selection by name
                Err(_) => {
                    let mut found:Option<&dyn MenuItem> = None;
                    for item in &self.items {
                        if item.get_display_string().eq(&selection) {
                            found = Some(*item)
                        }
                    }
                    found
                }
            };


            match selected_item {
                None => {
                    print_with(nest_level * INDENTATION, &format!("[31mUnrecognised item({}), try again: \n[0m", selection)).expect("Fatal print error");
                },
                Some(selected_item) => {
                    selected_item.notify_selected(nest_level + 1, Some(self)); //if selected item is a menu, it will block until it returns
                }
            };
        }
    }

    fn collect_inputs(&self) -> HashMap<String, String> {
        let mut all_sub_inputs = HashMap::new();
        for item in &self.items {
            let mut subs = item.collect_inputs();
            for (k, v) in subs.drain() {
                all_sub_inputs.insert(k, v);
            }
        }
        return all_sub_inputs
    }
}



pub trait InputItem<T: Display>: MenuItem {
    fn has_value(&self) -> bool;
    fn get_value(self) -> Option<T>;
    fn get_value_state_as_string(&self) -> String;
    fn clear(&mut self);
    fn run_for_value(self) -> Option<T> where Self: std::marker::Sized {
        self.notify_selected(0, None);
        self.get_value()
    }
}

pub type ValidatedInput<'a, T> = ParsedInput<'a, T>;
pub struct ParsedInput<'a, T: Display> {
    name: &'a str,
    parser: Box<dyn for<'b> Fn(&'b str, Option<&dyn MenuItem>) -> Result<T, &'b str>>,
    current_value: RefCell<Option<T>>
}
impl<T: Display> ParsedInput<'_, T> {
    pub fn new<'a>(name: &'a str, parser: impl for<'b> Fn(&'b str, Option<&dyn MenuItem>) -> Result<T, &'b str> + 'static) -> ParsedInput<'a, T> {
        ParsedInput {
            name,
            parser: Box::new(parser),
            current_value: RefCell::new(None)
        }
    }
}
impl<'a, T: Display> InputItem<T> for ParsedInput<'a, T> {
    fn has_value(&self) -> bool {
        self.current_value.borrow().is_some()
    }
    fn get_value(self) -> Option<T> {
        self.current_value.into_inner()
    }
    fn get_value_state_as_string(&self) -> String {
        return match self.current_value.borrow().as_ref() {Some(t)=> format!("{}", t), None => "None".to_string()};
    }
    fn clear(&mut self) {
        self.current_value.replace(None);
    }
}
impl<'a, T: Display> MenuItem for ParsedInput<'a, T> {
    fn get_display_string(&self) -> String {
        return format!("{} (= \"{}\")", self.name, self.get_value_state_as_string());
    }

    fn notify_selected(&self, nest_level: usize, parent_menu: Option<&dyn MenuItem>) -> bool {
        print_with(nest_level * INDENTATION, &self.get_display_string()).expect("Fatal print error");
        print_f_with(nest_level * INDENTATION, "Enter new value: ").expect("Fatal print error");
        let read = read_line().expect("Fatal print error");
        match (self.parser)(&read, parent_menu) {
            Ok(value) => {
                self.current_value.replace(Some(value));
                true
            }
            Err(why) => {
                print(&format!("[31m!!! Could not accept({}), keeping \"{}\"\n[0m", why, &self.get_value_state_as_string())).expect("Fatal print error");
                false
            }
        }
    }

    fn collect_inputs(&self) -> HashMap<String, String> {
        if let Some(val) = self.current_value.borrow().as_ref() {
            map!(self.name.to_string() => format!("{}", val))
        } else {
            HashMap::with_capacity(0)
        }
    }
}

pub type StringInput<'a> = TypeInput<'a, String>;
pub struct TypeInput<'a, F: FromStr+Display> {
    pub name: &'a str,
    pub current_value: RefCell<Option<F>>
}
impl<'a, T: FromStr+Display> TypeInput<'a, T> {
    pub fn clear(&mut self) {
        self.current_value.replace(None);
    }
    pub fn get_value(self) -> Option<T> {
        self.current_value.into_inner()
    }
    fn get_value_state_as_string(&self) -> String {
        return match self.current_value.borrow().deref() {Some(t)=> format!("{}", t), None => "None".to_string()};
    }

    pub fn new(name : &str) -> TypeInput<T> {
        TypeInput {
            name,
            current_value: RefCell::new(None)
        }
    }
}
impl<T: FromStr+Display> InputItem<T> for TypeInput<'_, T> {
    fn has_value(&self) -> bool {
        self.current_value.borrow().is_some()
    }
    fn get_value(self) -> Option<T> {
        self.current_value.into_inner()
    }
    fn get_value_state_as_string(&self) -> String {
        return match self.current_value.borrow().as_ref() {Some(t)=> format!("{}", t), None => "None".to_string()};
    }
    fn clear(&mut self) {
        self.current_value.replace(None);
    }
}
impl<T: FromStr+Display> MenuItem for TypeInput<'_, T> {
    fn get_display_string(&self) -> String {
        return format!("{} (Input=\"{}\")", self.name, self.get_value_state_as_string());
    }

    fn notify_selected(&self, nest_level: usize, _parent_menu: Option<&dyn MenuItem>) -> bool {
        print_with(nest_level * INDENTATION, &self.get_display_string()).expect("Fatal print error");
        print_f_with(nest_level * INDENTATION, "Enter new value: ").expect("Fatal print error");
        let read = read_line().expect("Fatal print error");
        match read.parse::<T>() {
            Ok(value) => {
                self.current_value.replace(Some(value));
                true
            },
            Err(_) => {
                print(&format!("[31m!!! Input could not be parsed (keeping \"{}\")\n[0m", &self.get_value_state_as_string())).expect("Fatal print error");
                false
            }
        }
    }

    fn collect_inputs(&self) -> HashMap<String, String> {
        if let Some(val) = self.current_value.borrow().as_ref() {
            map!(self.name.to_string() => format!("{}", val))
        } else {
            HashMap::with_capacity(0)
        }
    }
}

pub struct Choice<'a> {
    pub name: &'a str,
    choices: Vec<&'a str>,
    external_validation: Box<dyn Fn(isize, Option<&dyn MenuItem>) -> bool>,
    current_choice: Cell<isize>
}
impl Choice<'_> {
    pub fn get_selection_i(&self) -> isize {
        self.current_choice.get()
    }

    pub fn standalone<'a>(name : &'a str, choices: Vec<&'a str>) -> Choice<'a> {
        Choice {
            name,
            choices,
            external_validation: Box::new(|_,_| true),
            current_choice: Cell::new(-1)
        }
    }
    pub fn new<'a>(name : &'a str, choices: Vec<&'a str>) -> Choice<'a> {
        Choice {
            name,
            choices,
            external_validation: Box::new(|_,_| true),
            current_choice: Cell::new(-1)
        }
    }
    pub fn new_with_default<'a>(name : &'a str, choices: Vec<&'a str>, default_choice: isize) -> Choice<'a> {
        Choice {
            name,
            choices,
            external_validation: Box::new(|_,_| true),
            current_choice: Cell::new(default_choice)
        }
    }
    pub fn new_with_validation<'a>(name : &'a str, choices: Vec<&'a str>, default_choice: isize, external_validation: impl Fn(isize, Option<&dyn MenuItem>) -> bool + 'static) -> Choice<'a> {
        Choice {
            name,
            choices,
            external_validation: Box::new(external_validation),
            current_choice: Cell::new(default_choice)
        }
    }
}
impl InputItem<String> for Choice<'_> {
    fn has_value(&self) -> bool {
        let current_choice = self.current_choice.get();
        current_choice >= 0 && (current_choice as usize) < self.choices.len()
    }
    fn get_value(self) -> Option<String> {
        let current_choice = self.current_choice.get();
        if current_choice < 0 || current_choice as usize >= self.choices.len() {
            None
        } else {
            Some(self.choices[current_choice as usize].to_string())
        }
    }
    fn get_value_state_as_string(&self) -> String {
        let current_choice = self.current_choice.get();
        if current_choice < 0 || current_choice as usize >= self.choices.len() {
            "None".to_string()
        } else {
            self.choices[current_choice as usize].to_string()
        }
    }
    fn clear(&mut self) {
        self.current_choice.set(-1);
    }
}
impl MenuItem for Choice<'_> {
    fn get_display_string(&self) -> String {
        return format!("{} (Choice={})", self.name, self.get_value_state_as_string());
    }

    fn notify_selected(&self, nest_level: usize, parent_menu: Option<&dyn MenuItem>) -> bool {
        loop {
            println_with(nest_level * INDENTATION, &self.get_display_string()).expect("Fatal print error");
            let previous_choice = self.current_choice.get();
            let choices_length = self.choices.len();
            for (i, choice) in self.choices.iter().enumerate() {
                if previous_choice >= 0 && i == previous_choice as usize {
                    print_with(nest_level * INDENTATION, &format!("[33m-> {}: [0m{} [32m(X)[0m\n", i+1, choice)).expect("Fatal print error");
                } else {
                    print_with(nest_level * INDENTATION, &format!("-> {}: {}\n", i+1, choice)).expect("Fatal print error");
                }
            }
            print_with(nest_level * INDENTATION, &format!("-> {}: {}\n", choices_length + 1, "Cancel")).expect("Fatal print error");
            print_f_with(nest_level * INDENTATION, "Select choice(by number or name): ").expect("Fatal print error");

            let selection = read_line().expect("Fatal print error");
            let selected_item = match selection.parse::<isize>() {
                Ok(number) => {
                    number - 1
                }
                //selection by name
                Err(_) => {
                    let mut found:isize = -1;
                    for (i, choice) in self.choices.iter().enumerate() {
                        if choice.eq(&selection) {
                            found = i as isize
                        }
                    }
                    found
                }
            };

            if selected_item < 0 || (selected_item as usize) > choices_length {
                print_with(nest_level * INDENTATION, &format!("[31mUnrecognised choice({}), try again: \n[0m", selection)).expect("Fatal print error");
            } else {
                return if (selected_item as usize) < choices_length &&
                                                    (self.external_validation)(selected_item, parent_menu) {
                    self.current_choice.replace(selected_item);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn collect_inputs(&self) -> HashMap<String, String> {
        if self.has_value() {
            map!(self.name.to_string() => self.get_value_state_as_string())
        } else {
            HashMap::with_capacity(0)
        }
    }
}

pub struct ChoiceConstrainedInput<'a, T: Display> {
    preliminary_choice: Choice<'a>,
    input: ParsedInput<'a, T>
}
impl <T: Display>ChoiceConstrainedInput<'_, T> {
    pub fn new<'a>(name: &'a str, choices: Vec<&'a str>, parser: impl for<'b> Fn(&'b str, &str) -> Result<T, &'b str> + 'static) -> ChoiceConstrainedInput<'a, T> {
        ChoiceConstrainedInput {
            preliminary_choice: Choice::standalone(name, choices),
            input: ParsedInput {
                name,
                current_value: RefCell::new(None),
                parser: Box::new(move |raw, choice| {
                    if let Some(choice) = choice { //always successful
                        if let Some(choice) = choice.collect_inputs().values().next() { //choice is always something, because we validate that in choice constrained input
                            return (parser)(raw, choice);
                        }
                    }
                    Err("impossible")
                }),
            }
        }
    }
}
impl <T: Display> InputItem<DisplayableTuple<String, T>> for ChoiceConstrainedInput<'_, T> {
    fn has_value(&self) -> bool {
        self.input.has_value()
    }
    fn get_value(self) -> Option<DisplayableTuple<String, T>> {
        if let Some(choice) = self.preliminary_choice.get_value() {
            if let Some(input) = self.input.get_value() {
                return Some(DisplayableTuple::new(choice, input));
            }
        }
        None
    }
    fn get_value_state_as_string(&self) -> String {
        format!("{}->{}", self.preliminary_choice.get_value_state_as_string() , self.input.get_value_state_as_string())
    }
    fn clear(&mut self) {
        self.preliminary_choice.clear();
        self.input.clear();
    }
}
impl <T: Display>MenuItem for ChoiceConstrainedInput<'_, T> {
    fn get_display_string(&self) -> String {
        return format!("{} ({}->{})", self.preliminary_choice.name, self.preliminary_choice.get_value_state_as_string(), self.input.get_value_state_as_string());
    }

    fn notify_selected(&self, nest_level: usize, parent_menu: Option<&dyn MenuItem>) -> bool {
        let previous_choice = self.preliminary_choice.get_selection_i();
        if self.preliminary_choice.notify_selected(nest_level, parent_menu) {
            println_2_with(nest_level * INDENTATION, "Enter Input For Selected Choice: ", &self.preliminary_choice.get_value_state_as_string()).expect("Fatal print error");
            if self.input.notify_selected(nest_level + 1, Some(&self.preliminary_choice)) {
                true
            } else {
                self.preliminary_choice.current_choice.set(previous_choice);
                false
            }
        } else {
            false
        }
    }

    fn collect_inputs(&self) -> HashMap<String, String> {
        let mut map = HashMap::with_capacity(2);
        for (k, v) in self.preliminary_choice.collect_inputs().drain() {
            map.insert(k, v);
        }
        for (k, v) in self.input.collect_inputs().drain() {
            map.insert(k, v);
        }
        map
    }
}



pub type ExistingFilePathInput<'a> = ParsedInput<'a, String>;
impl ExistingFilePathInput<'_> {
    pub fn new_efp(name: &str) -> ExistingFilePathInput {
        ExistingFilePathInput {
            name,
            parser: Box::new(|raw, _| {
                if Path::new(raw).is_file() {
                    Ok(raw.to_string())
                } else {
                    Err("could not open file path")
                }
            }),
            current_value: RefCell::new(None)
        }
    }
}
pub type DirPathInput<'a> = ParsedInput<'a, String>;
impl DirPathInput<'_> {
    pub fn new_dp(name: &str) -> DirPathInput {
        DirPathInput {
            name,
            parser: Box::new(|raw, _| {
                if Path::new(raw).is_dir() {
                    Ok(raw.to_string())
                } else {
                    Err("file path invalid (parent dir does not exist)")
                }
            }),
            current_value: RefCell::new(None)
        }
    }
}
pub type NonExistingPathInput<'a> = ParsedInput<'a, String>;
impl NonExistingPathInput<'_> {
    pub fn new_nep(name: &str) -> NonExistingPathInput {
        NonExistingPathInput {
            name,
            parser: Box::new(|raw, _| {
                let path = Path::new(raw);
                if let Some(parent_path) = path.parent() {
                    if parent_path.is_dir() {
                        return Ok(raw.to_string());
                    }
                }
                Err("file path invalid (parent dir does not exist)")
            }),
            current_value: RefCell::new(None)
        }
    }
}


pub struct ExecutableItem<'a> {
    name: &'a str,
    executable: Box<dyn Fn(&str, Option<&dyn MenuItem>) -> bool>,
}
impl ExecutableItem<'_> {
    pub fn new<'a>(name: &'a str, executable: Box<dyn Fn(&str, Option<&dyn MenuItem>) -> bool>) -> ExecutableItem<'a> {
        ExecutableItem {
            name,
            executable: Box::new(executable)
        }
    }
}
impl MenuItem for ExecutableItem<'_> {
    fn get_display_string(&self) -> String {
        format!("{} (execute)", self.name)
    }
    fn notify_selected(&self, nest_level: usize, parent_menu: Option<&dyn MenuItem>) -> bool {
        println_2_with(nest_level * INDENTATION, "Executing ", self.name).expect("Fatal print error");
        (self.executable)(self.name, parent_menu)
    }
    fn collect_inputs(&self) -> HashMap<String, String> {
        HashMap::with_capacity(0)
    }
}


//HELPER
pub fn print(str: &str) -> io::Result<usize> {
    io::stdout().write(str.as_bytes())
}
pub fn print_f(str: &str) -> io::Result<()> {
    print(str)?;
    io::stdout().flush()
}
pub fn print_f2(str1: &str, str2: &str) -> io::Result<()> {
    print(str1)?;
    print_f(str2)
}
pub fn print_f3(str1: &str, str2: &str, str3: &str) -> io::Result<()> {
    print(str1)?;
    print(str2)?;
    print_f(str3)
}

pub fn print_spaces(number: usize) -> io::Result<()> {
    for _ in 0..number {
        io::stdout().write(&[32])?;
    }
    io::Result::Ok(())
}
pub fn println() -> io::Result<usize> {
    io::stdout().write(&[10, 13])
}
pub fn println_with(leading_spaces: usize, str: &str) -> io::Result<usize> {
    print_spaces(leading_spaces)?;
    print(str)?;
    println()
}
pub fn println_2_with(leading_spaces: usize, str1: &str, str2: &str) -> io::Result<usize> {
    print_spaces(leading_spaces)?;
    print(str1)?;
    print(str2)?;
    println()
}
pub fn print_with(leading_spaces: usize, str: &str) -> io::Result<usize> {
    print_spaces(leading_spaces)?;
    print(str)
}
pub fn print_f_with(leading_spaces: usize, str: &str) -> io::Result<()> {
    print_spaces(leading_spaces)?;
    print_f(str)
}
pub fn print_f2_with(leading_spaces: usize, str1: &str, str2: &str) -> io::Result<()> {
    print_spaces(leading_spaces)?;
    print_f2(str1, str2)
}
pub fn print_f3_with(leading_spaces: usize, str1: &str, str2: &str, str3: &str) -> io::Result<()> {
    print_spaces(leading_spaces)?;
    print_f3(str1, str2, str3)
}

pub fn read_line() -> Result<String, Box<dyn error::Error>> {
    let mut s=String::new();
    io::stdin().read_line(&mut s)?;
    if s.ends_with("\n") { s.pop(); }//to remove tailing linebreak
    if s.ends_with("\r") { s.pop(); }//to remove tailing linebreak
    return Ok(s);
}

pub fn print_and_read_line(message: &str) -> Result<String, Box<dyn error::Error>> {
    if !message.is_empty() {
        print_f(message)?;
    }
    read_line()
}
pub fn print_and_read_line_as_i32(message: &str) -> Result<i32, Box<dyn error::Error>> {
    return Ok(print_and_read_line(message)?.parse::<i32>()?)
}