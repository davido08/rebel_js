/*This project is an open source JavaScript/HTML/CSS compressor
meant to drastically reduce the size of many websites.

It allows develoopers to write verbose, concise and clear code in developpment,
and get short and optimized code in production effortlessly and super quickly.

NOTE: This program is still in Beta and is open source, so if you notice any bugs, possible optimizations
or parts of the code that could be improved, don't hesitate to hit me up on my GitHub: --insert github link--

The logic of the code is moderately complex, but i will try my best to explain through the code and 
more in depth in the documentation available on the --insert file name-- file.


- David Firlotte, 2025

*/

mod characters;
use core::panic;
use std::collections::binary_heap::Iter;
use std::collections::HashMap;
use std::fmt::Error;
use std::process::exit;
use std::thread::current;
use tokio::{fs::File, io::AsyncReadExt};
use tokio::sync::mpsc;
use std::{io::Read, str};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use std::rc::Rc;
use std::cell::RefCell;
use regex::Regex;
use std::env::{self, set_current_dir};





#[derive(Clone, PartialEq, Eq, Hash)]


//Establishing constans in order for the code to be more understandable 




//Error declarations 

struct InvalidSyntaxError;

impl fmt::Display for InvalidSyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Code given as an input seems to contain a sytax error.
        Please make sure that the code served as an input is 100% operational.
        If this seems like a mistake, note that this program is currently under development and we would like to get your feedback as soon as possible.
        You can report bugs or unusual behaviour on our -----INSERT CONTACT PAGE HERE------")
    }
}

struct CharacterAmounts{
    one_byte_chars: [u32;127],
    two_byte_chars: [Vec<(u8, u16)>;255],
    three_byte_chars: [Vec<([u8;2], u16)>;255],
    four_byte_chars: [Vec<([u8;3], u16)>;255]
}


struct BitSet64 {
    bitset: u64,
    offset: u8,
}

impl BitSet64 {
    fn insert(&mut self, position: u8) {
        self.bitset = self.bitset | (1 << (position - self.offset))
    }
    fn remove(&mut self, position: u8) {
        self.bitset = self.bitset & !(1 << (position - self.offset));
    }
    fn get(&self, position: u8) -> bool {
        return 1 & (self.bitset >> (position - self.offset)) == 1;
    }
}


struct Buffers<'a> {
    previous_buffer: [u8;1024],
    current_buffer: [u8;1024],
    buffer_ref: &'a [u8;1024],
    going_to_current:bool,
    last_index: usize,
}

impl<'a> Buffers<'a> {
    fn check_index(&mut self, i_buffer: &mut usize)-> bool {
        if *i_buffer == 1024 {
            if !self.going_to_current {
                self.going_to_current = true;
                self.buffer_ref = &self.current_buffer;
                *i_buffer=0;
            } 
            self.going_to_current = !self.going_to_current;
            return self.going_to_current;
        }
        return true;
    }
}

struct PotentialValue {
    name: Vec<u8>,
    is_a_function:bool,
    blocked_values: [bool;58],

}

impl PotentialValue {
    fn new(name: Vec<u8>, is_a_function: bool, blocked_values_vec: Vec<NewValueName>) -> Self {
        let mut blocked_values = [false;58];
        for value in blocked_values_vec {
            match value {
                NewValueName::OneChar(x) => blocked_values[(x - 65) as usize],
                _=>{}
            }
        }
        PotentialValue {
            name,
            is_a_function,
            blocked_values: [false;58],
        }
    }
}



enum NewValueName {
    OneChar(u8), 
    TwoChar([u8;2]),
}

#[derive(Clone, PartialEq)]
enum EndOfScopeChar {
    Curly,
    Parenthesis,
    SemiColon,
    None,
}

enum MatchType {
    Done(bool),
    ToContinue(Vec<u8>),
}
#[derive(PartialEq)]
enum PreviousLineState {
    Code, 
    JSComment,
    HTMLComment, 
    StringConcat([u8;2]),
    WaitingParenthesis,
    WaitingEndScopeChar(u8),
    ExpectingScopeChar,
    FindingIfCorresponds(Vec<u8>),

}
#[derive(Clone, PartialEq)]
enum InsideOf {
    NormalCode,

    //Here the first value wrapped is the new value name of the function, the second one is its list of undeclared yet values.
    Function(NewValueName),

    //This is not a real scope but represents the integrity of an if/else if/else statement. 
    //The vector of JsScopes will contain all of the "children" scopes (one for if, other one for else if and last one for else)

    //The boolean will turn true if one of the child scopes is obligated to have curlies as end scope character,
    // which means that all of the child scopes will have curlies as the end of scope character.
    UpperIfStatement(Vec<JsScope>, bool),
    IfStatement,
    IfElseStatement,
    ElseStatement,
    ForLoop,
    Undefined,
}
enum GoingThrough {
    HTML,
    CSS, 
    JS
}
struct FnParams<'a> {
    i: &'a mut usize,
    end_index: &'a mut usize,
    buf_bytes: &'a mut Vec<u8>,
    value_name: &'a mut Vec<u8>,
    all_values_count: &'a mut HashMap<NewValueName, u16>,
    last_index: &'a mut usize,
    current_scope: &'a mut JsScope,

}
impl<'a> FnParams<'a> {
    fn return_indexes(&mut self, indexes:[usize; 3])  
    {
        [*self.end_index, *self.i, *self.last_index] = indexes;
    }
}

struct DuoValueFunc {
    func: [fn (&mut FnParams); 2],
}

#[derive(Clone, PartialEq, Eq)]
struct Value {
    value_old_name: Vec<u8>,
    value_new_name: NewValueName,
    amount_occurences: u16,
    declaration_index: usize,
    last_usage_index: usize,
    function_index: Option<usize>,

}


#[derive(Clone, PartialEq)]
struct JsScope {
    values: Vec<Value>,
    used_values: Vec<Value>,
    parent_scope: Option<Rc<RefCell<JsScope>>>,
    starting_index: usize,
    latest_available_one_byte: u8,
    children_used_values: Vec<Value>,
    characters: Vec<u8>,
    inside_of: InsideOf,
    end_scope_char: EndOfScopeChar,
    min_value_declarations: u16,
    all_value_declarations: u16,
    semi_column_indexes: Vec<usize>,
    is_a_parent: bool,
    beginning_chars: Vec<u8>,
    potential_values_called: Vec<PotentialValue>,
    potential_values_declared: Vec<(NewValueName,Vec<PotentialValue> )>,
    inside_function: Option<NewValueName>,
    
}
impl JsScope {
    pub fn new(parent: Option<JsScope>, inside_of: InsideOf) -> Self {
        let (one_byte, current_index): (u8, usize) = 
        if let Some(scope) = &parent {
            (scope.latest_available_one_byte, scope.starting_index+scope.characters.len())
        } else {(65, 0)};
        JsScope {
            values: Vec::new(),
            used_values: Vec::new(),
            parent_scope: parent.map(|s| Rc::new(RefCell::new(s))),
            starting_index: current_index,
            latest_available_one_byte: one_byte,
            children_used_values: Vec::new(),
            characters: Vec::new(),
            inside_of: inside_of,
            end_scope_char: EndOfScopeChar::None,
            min_value_declarations: 0,
            all_value_declarations: 0,
            semi_column_indexes: Vec::new(), 
            is_a_parent: false,
            beginning_chars: Vec::new(),
            potential_values_called: Vec::new(),
            inside_function: match inside_of { 
                InsideOf::Function(x) => Some(x),
                _=>  
                 match parent {
                    Some(p) => p.inside_function,
                    None=>None
                }
            }

        }
    }
    pub fn new_value(&mut self, old_value: Vec<u8>, current_index: usize) -> Value {

        //Find if the declared value has been used before in the code (Like in a function that hasn't been called before value declaration)
        let mut i=0;

        while i < self.potential_values_declared.len() {
            let potential_value = self.potential_values[i];
            if potential_value.name == old_value {

             //pass by all blocked values

             while self.latest_available_one_byte != 122 
             && potential_value.blocked_values[(self.latest_available_one_byte - 65 ) as usize] {
                self.latest_available_one_byte+=1;
             }

            }
            i+=1;
        }
        
        let new_name: NewValueName = match self.latest_available_one_byte {
            90 => {
                self.latest_available_one_byte = 97;
                NewValueName::OneChar(97)
               
            } 
            122 => {
                let mut two_byte: [u8;2] = [65,65];
                
                loop {
                    let mut value_there = false;
                     for value in &self.values {
                            if let NewValueName::TwoChar(val_two) = &value.value_new_name {
                                if *val_two == two_byte  {
                                    value_there = true;
                                    match two_byte[1] {
                                        90 => {
                                            two_byte[1] = 97;
                                        } 
                                        122 => {
                                            match two_byte[0] {
                                                90 => {
                                                    two_byte[0] = 97;
                                                    two_byte[1] +=1;
                                                }
                                                122 => {
                                                    panic!("Either my code doesnt behave like its supposed to, or there is actually over 2500 values in the same scope which is beyond insane");
                                                } 
                                                _ => {
                                                    two_byte[0] += 1;
                                                    two_byte[1] = 65;
                                                }
                                            }
                                            
                                        } 
                                        _ => {
                                            two_byte[1]+=1;
                                        }
                                    }
                                } 
                            }
                    }
                    if !value_there {
                        break;
                    }
                }
                NewValueName::TwoChar(two_byte)

               
            } _ => {
                self.latest_available_one_byte+=1;
                NewValueName::OneChar(self.latest_available_one_byte)

            }

        };

       let new_value = Value {
            value_old_name: old_value,
            value_new_name : new_name,
            amount_occurences: 0,
            declaration_index: current_index,
            last_usage_index: current_index,
            function_index: if self.is_function {
                Some(self.starting_index)
            } else {
                None
            },

        };
        self.values.push(new_value.clone());
        
        return new_value;
        
    }

    pub fn add_char(value: u8, )

}

const MATCHES: [&str; 22] = [
    "function ",
    "let ",
    "const ",
    "var ",
    "else if",
    "async ",
    "await ",
    "return ",
    "else ",
    "yield ",
    "instanceof ",
    "typeof ",
    "void ",
    "delete ",
    " in ",
    " of ",
    "new ",
    "throw ",
    "case ",
    " class=",
    " id=",
    " onclick="

];

fn find_match() {
    
}

fn value_name_is_valid(a: u8, b: u8) -> bool {
    match a {
        0..=47 | 59..=64 | 91..=96 | 123..=127 => {
            match b {
                0..=47 | 59..=64 | 91..=96 | 123..=127 => {
                    true
                }
                _ => {
                    false
                }
            }
        }
        _ => {
            false
        }
    }
}

fn find_value_to_replace(q_char: &mut u8, j: &mut usize, changed_values: &HashMap<[u8;2], u8>, current_scope: &mut JsScope) {
    let mut jdx = *j;
    let mut q_char_dx= *q_char;
    match q_char_dx {
        39 | 34 => {
            if current_scope.characters[jdx] == q_char_dx {
                q_char_dx=0;
            }
        }
        96 => {
            match current_scope.characters[jdx] {
                96=> {
                    q_char_dx=0;
                }
                36 if current_scope.characters[jdx+1] == 123=> {
                        jdx+=2;
                        while current_scope.characters[jdx] != 125 {
                            let mut new_q_char: u8 = 0;
                            find_value_to_replace(&mut new_q_char, &mut jdx,  changed_values, current_scope);
                            jdx+=1;
                        }
                        jdx+=1;
                }
                _=> {}
            }
        
        } 
        _ => {
            for (old_val, new_val) in changed_values {
                if current_scope.characters[jdx] == old_val[0] && 
                   current_scope.characters[jdx+1] == old_val[1] &&
                   value_name_is_valid(current_scope.characters[jdx-1], current_scope.characters[jdx+2]) 
                {
                    current_scope.characters.remove(jdx+1);
                    current_scope.characters[jdx] = *new_val;

                }
            
            }
        }

    }

    *j=jdx;
    *q_char=q_char_dx;

}


fn chose_function(last_index: &mut usize,
        i: &mut usize,
        increment: usize,
        wanted_function:
        DuoValueFunc,
        current_function: 
        &mut DuoValueFunc
)-> bool
{
    *i+=increment;
    *last_index+=increment;
    *current_function = wanted_function;
    return true;
}


fn const_fn(mut fn_params: &mut FnParams) 
{
    let mut e_i = *fn_params.end_index;
    let mut idx = *fn_params.i;
    let mut l_i = *fn_params.last_index;
    loop {
        e_i+=1;
        match fn_params.buf_bytes[e_i] {
            61 | 32   => {
                let value = fn_params.current_scope.new_value( fn_params.value_name.clone(), idx);
                idx+=1;
    
                match value.value_new_name {
                    NewValueName::OneChar(val) => {
                        e_i-=1;
                        fn_params.buf_bytes[idx] = val 
                    }
                    NewValueName::TwoChar(val) => {
                        e_i-=2;
                        (fn_params.buf_bytes[e_i], fn_params.buf_bytes[e_i+1]) = (val[0], val[1]);
                    }
                }
                fn_params.buf_bytes.drain(idx..e_i);
                
                fn_params.all_values_count.insert(value.value_new_name.clone(), 1);
                fn_params.current_scope.used_values.push(value.clone());
                break;
            } 
            _ => {
                fn_params.value_name.push(fn_params.buf_bytes[idx]);
    
            }
        }

    }
    fn_params.return_indexes([e_i, idx, l_i]);

}

fn find_function_name(buffers: &Buffers, current_scope: &mut JsScope, i_buffer: &mut usize, all_values_count: &mut HashMap<NewValueName, u16>){
    let mut function_name: Vec<u8> = Vec::new();

    
    let value = {
        loop {
            i_buffer+=1;
            
            match *buffers.buffer_ref[i_buffer] {
                32 | 40 =>{
                    break; 
                    
                }
                x=> function_name.push(x)
            }
        }
        add_value(&mut current_scope, &mut all_values_count, function_name)         
        
    };

    while buffers.buffer_ref[i_buffer]!=40{i_buffer+=1;}

    let mut function_param: Vec<u8> = Vec::new();
    current_scope = JsScope::new(Some(current_scope),   InsideOf::Function(value.value_new_name));
}


fn new_values_init(buffers: &Buffers, current_scope: &mut JsScope, i: &mut usize, all_values_count: &mut HashMap<NewValueName, u16>) {

    let mut param_name: Vec<u8> = Vec::new();
    let mut new_values: Vec<NewValueName>;
    loop {
        match *buffers.buffer_ref[i] {
            32 | 10 => {},
            40 | 61 => {
                new_values.push(add_value(current_scope,  all_values_count,  std::mem::take(&mut param_name)).value_new_name);
                break;
            }
            44 => new_values.push(add_value(current_scope,  all_values_count,  std::mem::take(&mut param_name)).value_new_name),
            x=>param_name.push(x),
        }
        *i+=1;
    }
   
    if new_values.len()>1 {
        current_scope.characters.push(40);
        for value_name in new_values {
            match value_name {
                NewValueName::OneChar(x) =>current_scope.characters.push(x),
                NewValueName::TwoChar(x) => {
                    for char in x {
                        current_scope.characters.push(char)
                    }
                }
            }
            current_scope.characters.push(44);
        }
        *current_scope.characters.last_mut().unwrap() = 41;
    }
    else {
        match new_values[0] {
            NewValueName::OneChar(x) => current_scope.characters.push(x),
            NewValueName::TwoChar(x) => {
                for char in x {
                    current_scope.characters.push(char);
                }
            }
        }
        
    }

}

fn let_fn(mut fn_params: &mut FnParams) 
{
    let mut idx = *fn_params.i;
    let mut l_i = *fn_params.last_index;

    let mut length_new_values=0;
    
    let mut values_list: Vec<Value> = Vec::new();
    loop {
        idx+=1;
        let char = fn_params.buf_bytes[idx];
        match char {
            32 => {}
            61 | 44 => {
                
                let value = fn_params.current_scope.new_value( std::mem::take(fn_params.value_name), idx);
                
                fn_params.all_values_count.insert(value.value_new_name.clone(), 1);
                fn_params.current_scope.used_values.push(value.clone());
                if char != 44 {
                    let mut index= idx-length_new_values;
                    match value.value_new_name {
                        NewValueName::OneChar(x) => {
                            fn_params.buf_bytes[index-1] = x;
                            index-=1;
                        }
                        NewValueName::TwoChar(x) => {
                            [fn_params.buf_bytes[l_i], fn_params.buf_bytes[index+1]] = x;
                            index-=2;
                        }
                    }
                    fn_params.buf_bytes.drain(l_i..index);

                    loop {
                        l_i+=1;
                        fn_params.buf_bytes[l_i] = 44;
                        l_i+=1;

                        match values_list.pop() {
                            Some(val)=> {
                                match val.value_new_name {
                                    NewValueName::OneChar(x) => {fn_params.buf_bytes[l_i] = x}
                                    NewValueName::TwoChar(x) => {
                                        [fn_params.buf_bytes[l_i], fn_params.buf_bytes[l_i+1]] = x;
                                        l_i+=1;
                                        
                                    }
                                }
                            }
                            None=>{break;}
                        }
                    }
                    break;
                }
               
                length_new_values += match value.value_new_name {
                    NewValueName::OneChar(_) => {2}
                    NewValueName::TwoChar(_) => {3}
                };
                 values_list.push(value);
                
                
            } 

            _ => {
                
                fn_params.value_name.push(fn_params.buf_bytes[idx]);
            }
        }

    }
    fn_params.return_indexes([idx, idx, idx-1]);

}

fn find_quote_chars(
    current_scope: &mut JsScope,
    i_buffer: &mut usize,
    quote_char: &mut u8, 
    previous_line_state: &mut PreviousLineState,
    going_through: &mut GoingThrough,
    buffers: &mut Buffers,
) {
    
    let byte = buffers.buffer_ref[*i_buffer];
    current_scope.characters.push(byte);
    if byte == *quote_char {
        
        *quote_char = 0;
       
    } else if *quote_char == 96  {

        if byte == 36 && find_if_corresponds(buffers.buffer_ref,  i_buffer, vec![123]) {
            *i_buffer+=1;
            
            look_for_values_js(buffers.buffer_ref, current_scope, i_buffer, true, previous_line_state, going_through );
        }

    } 
}

fn look_for_values_js( 
    current_scope: &mut JsScope,
    i_buffer: &mut usize,
    inside_concat_string: bool,
    previous_line_state: &mut PreviousLineState,
    going_through: &mut GoingThrough,
    buffers: &mut Buffers


) { 
    let mut idx =*i_buffer;
    let mut quote_char: u8 = 0;

    while idx < buffers.buffer_ref.len() {
        let byte = buffers.buffer_ref[*i_buffer];
        if quote_char != 0 {
            find_quote_chars(buffers.buffer_ref, current_scope, i_buffer, &mut quote_char, previous_line_state, going_through);
        } else {
            match byte {
                10 => {}

            }
        }
    }


}




fn find_value(
    buffers: &mut Buffers,
    current_scope: &mut JsScope,
    i_buffer: &mut usize,
    all_values_count: &mut HashMap<NewValueName, u16>
) {

    let mut idx = *i_buffer;
    let startindex = idx;
        let mut scope = Rc::new(RefCell::new(current_scope.clone()));
        let mut j = idx+1;



        while j< buffers.buffer_ref.len() {
            match buffers.buffer_ref[j] {
                0..=47 | 58..=64 | 91 | 93 | 94 | 123..=128 => {
                    break;
                }
                _=> {
                    j+=1;
                }
            }
        }

        let value_name = buffers.buffer_ref[startindex..j].to_vec();

        loop {
            let mut scope_ref = scope.borrow_mut();
            
                for value in scope_ref.values.iter_mut() {
                    if value.value_old_name == value_name {

                        //Put the value in the blocked values of the potential values
                        for potential_value in &mut current_scope.potential_values_called {
                            if potential_value.name == value_name {
                                match value.value_new_name {
                                    NewValueName::OneChar(byte) => {
                                        let index = (byte - 65) as usize; // Convert A-Z to 0-25
                                        if index < 58 {
                                            potential_value.blocked_values[index] = true;
                                        }
                                    }
                                    NewValueName::TwoChar(_) => {
                                        // Handle two-char case if needed
                                        // For now, skip as blocked_values only has 58 slots
                                    }
                                }
                            }
                        }


                        let mut contained = false;
                        for val in &mut current_scope.used_values {
                            if val.value_new_name == value.value_new_name {
                                val.last_usage_index = idx;
                                val.amount_occurences+=1;
                                contained=true;
                            }
                        }

                        
                        if !contained {
                            current_scope.used_values.push(value.clone());
                            current_scope.children_used_values.push(value.clone());
                        }
                        value.amount_occurences +=1;
                        all_values_count.insert(
                            value.value_new_name.clone(),
                            *all_values_count.get(&mut value.value_new_name).unwrap() + 1,
                        );
                        match value.value_new_name {
                            NewValueName::OneChar(x)=> {current_scope.characters.push(x);}
                            NewValueName::TwoChar(x) => {current_scope.characters.append(&mut x.to_vec());}
                        }

                        current_scope.characters.push(buffers.buffer_ref[j]);
                        *i_buffer = j-1;
                        return;

                    }
                }
                            
                       
            // Traverse up the parent chain
            let next_scope = match &scope_ref.parent_scope {
                Some(parent) => Some(parent.clone()),
                None => break,
            };
            drop(scope_ref); // End the borrow before reassigning
            scope = next_scope.unwrap();
        }
        


        match value_name {
            //console | window | document | fetch |
            [99,111,110,115,111,108,101] | [119,105,110,100,111,119] | [100,111,99,117,109,101,110,116] | [102,101,116,99,104] => {
                
            } _ => {
                if !current_scope.potential_values_called.iter().any(|val| val.name == value_name) {
                    let is_function = {
                        let k = j;
                        while k < buffers.buffer_ref.len() {
                            match buffers.buffer_ref[k] {
                                32 => k+=1,
                                40 => true, 
                                _ => false
                                
                            }
                            false
                    }
                    
                };
                current_scope.potential_values_called.push(PotentialValue::new(value_name, is_function, current_scope.children_used_values));
            }
        }
    
    }
}





fn find_if_corresponds(buffers: &mut Buffers, i_buffer: &mut usize, match_next: Vec<u8>, previous_line_state: &mut PreviousLineState) -> bool {
    if *i_buffer+match_next.len() >= buffers.buffer_ref.len() {
         return false;
    } 
    if buffers.buffer_ref[*i_buffer+1..*i_buffer+match_next.len()+1] == match_next {
            *i_buffer+=match_next.len();
            return true;
    }
    
    return false;
    
}

fn check_else_statement(current_scope: &mut JsScope, i_buffer: &mut usize) {
    
}

fn assign_end_scope_char(current_scope: &mut JsScope, previous_line_state: &mut PreviousLineState, end_scope_char: EndOfScopeChar) {
    
    current_scope.beginning_chars = std::mem::take(&mut current_scope.characters); 
    current_scope.end_scope_char = end_scope_char;
    *previous_line_state = PreviousLineState::Code;
}

async fn scope_end(current_scope: &mut JsScope, all_values_count: &mut HashMap<NewValueName, u16>, previous_line_state: &mut PreviousLineState)-> io::Result<()> {

    current_scope.characters.push(125);
    if let Some(scope) = current_scope.parent_scope.clone() {
        let all_children_used_values = current_scope.children_used_values.clone();
        let mut unused_values: HashMap<u8, Option<usize>> = HashMap::new();

        for val in all_values_count.keys().cloned() {
            if let NewValueName::OneChar(one_char) = val {
                
                let mut unused=true;
                for value in &current_scope.values {
                    if value.value_new_name == val {
                        unused=false;
                        break;
                    }
                }

                if unused {
                    let mut present_after: bool = false;
                    for child_value in current_scope.children_used_values.clone() {
                        if child_value.value_new_name == val {
                            unused_values.insert(one_char, Some(child_value.last_usage_index));
                            present_after=true;
                            break;
                        }
                    }
                    if !present_after {
                        unused_values.insert(one_char, None);
                    }
                    
                }
            }
            
        }
        let mut changed_values: HashMap<[u8;2], u8> = HashMap::new();
        
        for value in &mut current_scope.values {
            if let NewValueName::TwoChar(two_char) =value.value_new_name {
            let mut changed_value : Option<u8> = None;
            
            for (new_val, index) in &unused_values {
                /*If the last time the value was called is before the assignment
                 of the value we want to replace, then we can go ahead and replace it
                 without affecting the functionnality of the code. */
                if match index {
                   Some(i)=>{*i<value.declaration_index }
                   None=>{true}
                }{
                    changed_values.insert(two_char, *new_val);
                    changed_value = Some(*new_val);
                    value.value_new_name = NewValueName::OneChar(*new_val);
                    break;
                }
                    
                }
                if let Some(v) = changed_value {
                    unused_values.remove(&v);
                }
            }
            tokio::task::yield_now().await;
        }

        let mut j = current_scope.starting_index;
        let mut q_char: u8 = 0;
        
        let len = 2;
        while j >= len {

            find_value_to_replace(&mut q_char, &mut j, &changed_values, current_scope);
            tokio::task::yield_now().await;
        }


        let mut children_min_value_declaration = current_scope.min_value_declarations;

        if current_scope.inside_of == InsideOf::ForLoop {
            children_min_value_declaration +=1;
        }

        let all_value_declarations = current_scope.all_value_declarations+current_scope.values.len() as u16;


        match current_scope.end_scope_char {
            EndOfScopeChar::Curly=>chars.push(123),
            EndOfScopeChar::Parenthesis=>chars.push(41),
            _=>{}
        }
        //Re-ordering big if/else statements

        match current_scope.inside_of {
            InsideOf::UpperIfStatement(list_scopes,_ )=> {
                let mut curlies = false;
                //Usde to determine if the whole statement has to be used with curlies and "if" and "else" keywords.
                for scope in list_scopes {
                    if scope.end_scope_char == EndOfScopeChar::Curly {
                        curlies=true;
                        break;
                    }
                }
                let multiple = list_scopes.len()>1;
                for scope in list_scopes {
                    //Code to run if the whole scope has to be in curlies
                   if curlies {
                        match scope.inside_of {
                            //Add the "if" keyowrd to the chars 
                            InsideOf::IfStatement=>current_scope.characters.append(&mut vec![105,102]),
                            //Add the "else if" keyword to the chars
                            InsideOf::IfElseStatement=>current_scope.characters.append(&mut vec![101,108,115,101,32,105,102]),
                            //Add the "else" keyword to the chars
                            InsideOf::ElseStatement=>current_scope.characters.append(&mut vec![101,108,115,101]),
                            //Supposed to be impossible to have another InsideOf type
                            _=>panic!("Wtf"),
                        }
                        

                        //Add (
                        current_scope.characters.push(40);

                        //Add the condition 
                        current_scope.characters.append(&mut scope.beginning_chars);

                        //Add )
                        current_scope.characters.push(41);

                        //Add {
                        current_scope.characters.push(123);

                        //Add the contents of the scope
                        current_scope.characters.append(&mut scope.characters);

                        //Add }
                        current_scope.characters.push(125);

                   }
                   //If the scope doesn't have to be in curlies, therefore we can use shortcuts in the syntax.

                   /*If multiple sub scopes are in the if/else statement (Not just one "if"),
                   then use the turnary syntax like this : "condition?code_if_true:code_if_false;"*/
                   else {
                    
                /*In an if/else scope like this, if there is multiple statements in the same scope,
                we seperate them with commas like a normal scope ending with a semi colon, but here we have 
                to add parenthesis. This code verifies if there is more than one sentence.*/

                 let multiple_statements = {
                    if scope.semi_column_indexes.len()>0 {

                        
                    true
                    }
                    else {
                        let mut insides=0;
                        let mut result =true;
                        for char in scope.characters {
                            match char {
                                40 | 91 => {

                                    insides+=1;
                                }
                                41 | 93 => {

                                    if insides==0 {
                                        return Err(InvalidSyntaxError);
                                    }
                                    insides-=1;
                                }
                                44 => {

                                    if insides==0 {
                                        result=false;
                                    }
                                }
                            }
                            

                        }
                        result
                    }
                 };

                 if multiple {
                    match scope.inside_of {

                        InsideOf::IfStatement=> {
                            //Condition
                            current_scope.characters.append(&mut scope.beginning_chars);

                            //"?" character (shortcut for "if" after the condition)
                            current_scope.characters.push(63);
                        }
                        InsideOf::IfElseStatement => {
                            //":" standing for "else"
                            current_scope.characters.append(58);

                            //Then putting another if statement

                            //Condition
                            current_scope.characters.append(&mut scope.beginning_chars);

                            //"?" character (shortcut for "if" after the condition)
                            current_scope.characters.push(63);

                            
                        } 
                        InsideOf::ElseStatement => {
                            current_scope.characters.push(58);
                        }

                    }
                    

                    /*If there is a single "if" statement, we simply use this syntax: "condition&&code;"

                    Using the and logic, if the first condition is false, it wont execute whatevers comes next.
                    Because of this, simply using the && with the code to execute after is the shortest substitue
                    for a simple if statement. */
                   } else {
                    
                    for index in scope.semi_column_indexes {
                        scope.characters[index]=44;
                    }
                    //Add condition
                    current_scope.characters.append(&mut scope.beginning_chars);

                    //Add "&&"
                    current_scope.characters.push(38);
                    current_scope.characters.push(38);

                    //Add code 


                   }
                   if multiple_statements{current_scope.characters.push(40);}
                    current_scope.characters.append(&mut scope.characters);
                    if multiple_statements{current_scope.characters.push(41);}
                   
                   
                }

            }
        }
                 
                   

        }


        let parent = scope.borrow();
        let mut parent_clone = parent.clone();
        drop(parent);


        //If the scope is a function, we need to add the potential values declared to the potential values declared.
        //Else, we need to add the potential values called to the potential values called,


        match current_scope.inside_of {
            InsideOf::Function(x) => parent_clone.potential_values_declared.push((x, current_scope.potential_values_called)),
            _=> parent_clone.potential_values_called.append(&mut current_scope.potential_values_called),
        };
        parent_clone.potential_values_declared.append(&mut current_scope.potential_values_declared);

         //If the parent scope is a grouped if/else statement 
         match &mut parent_clone.inside_of {
            InsideOf::UpperIfStatement(scopes,_ ) => {
                scopes.push(current_scope.clone());
            }
            _=> {
                parent_clone.characters.append(&mut current_scope.beginning_chars);
                parent_clone.characters.append(&mut current_scope.characters);
            }
         }
    
        
        *current_scope = parent_clone;

        current_scope.min_value_declarations += children_min_value_declaration;
        current_scope.all_value_declarations = all_value_declarations;

        current_scope.is_a_parent = true;


        current_scope.children_used_values = all_children_used_values;
            
     }
     Ok(())

}


fn add_value(current_scope: &mut JsScope, all_values_count: &mut HashMap<NewValueName, u16>, function_param: Vec<u8>) -> Value {
    let value = current_scope.new_value(function_param, current_scope.starting_index+current_scope.characters.len());
    all_values_count.insert(value.value_new_name.clone(), 1);
    current_scope.used_values.push(value.clone());
    
    return value;
}

fn drain_values(buf_bytes: &mut Vec<u8>, i: &mut usize, last_index: &mut usize) {
    if *last_index+1 != *i {
        buf_bytes.drain(*last_index+1..*i);
        *i=*last_index;
    } else {
        *last_index=*i;
    }
}

pub async fn compress_js(file_input: &str, file_output: &str) -> io::Result<()> {
    //1. Remove comments
    //2. Minimify functions/variable/class/element names so they can all be one byte character long if possible.

    let mut amount_occurences: [u16;128] = [0;128];
    let mut shortcut_eight_byte_chars: Vec<u8> = Vec::new();
    let mut free_eight_byte_characters: Vec<u8> = Vec::new();
    let mut content_bound_w_eight_bytes: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let mut content_occurences: Vec<u8> = Vec::new();

    let mut least_recurring_char: usize = 0;

    let mut larger_content: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let mut larger_shortcut: Vec<[u8;2]> = Vec::new();
    let mut taken_2_byte_chars: HashMap<[u8;2], u16> =  HashMap::new();
    let mut latest_available_2_byte: [u8;2] = [192, 128];

    let mut taken_3_byte_chars: HashMap<[u8;3], u16> =  HashMap::new();
    let mut taken_4_byte_chars: HashMap<[u8;4], u16> = HashMap::new();
    
    let mut calculate_amount = || {
        let mut all_texts: Vec<String> = [content_bound_w_eight_bytes.borrow().clone(), larger_content.borrow().clone()].concat();
        all_texts.push(buffer.borrow().clone());
        for text in all_texts {
            let mut i=0;
                    let bytes = text.to_string().clone().into_bytes();
                    let len = bytes.len();

                    while i < len {
                        let byte = bytes[i];
                        if byte < 128{
                            amount_occurences[byte as usize]+=1;
                            i+=1;
                        }
                        else if byte >= 192 && byte < 224 {
                            let character = [byte, bytes[i+1]];
                            taken_2_byte_chars.insert(character, match taken_2_byte_chars.get(&character){Some(x)=>x+1,None=>1});
                            i+=2;
                        } else if byte >= 224 && byte < 240 {
                            let character = [byte, bytes[i+1], bytes[i+2]];
                            taken_3_byte_chars.insert(character, match taken_3_byte_chars.get(&character){Some(x)=> *x+1,None=>1});
                            i+=3
                        } else {
                            let character = [byte, bytes[i+1], bytes[i+2], bytes[i+3]];
                            taken_4_byte_chars.insert(character, match taken_4_byte_chars.get(&character){Some(x)=> *x+1,None=>1});
                            i+=4;
                        }
            }
        }
    };

    calculate_amount();

    for i in 0..128{if amount_occurences[i]==0{free_eight_byte_characters.push(i as u8)}}
    let mut replace_everywhere = |from: &str, to: &str| {
        let mut buffer_ref = buffer.borrow_mut();
        *buffer_ref = buffer_ref.replace(from, to);

        let mut content_vec = content_bound_w_eight_bytes.borrow_mut();
        for s in content_vec.iter_mut() {
            *s = s.replace(from, to);
        }

        let mut larger_vec = larger_content.borrow_mut();
        for s in larger_vec.iter_mut() {
            *s = s.replace(from, to);
        }
    };
    for mat in MATCHES {
        if buffer.borrow().contains(mat) {
            match free_eight_byte_characters.pop() {
                Some(x) => {
                    let char = x;
                    replace_everywhere(mat,str::from_utf8(&[char]).unwrap());
                    shortcut_eight_byte_chars.push(char);
                    Rc::clone(&content_bound_w_eight_bytes).borrow_mut().push(mat.to_string());
                } None => {
                    least_recurring_char = 128;
                    let amount_mat = buffer.borrow().match_indices(mat).count() as u16;
                    let mut least_amount= amount_mat;
                    for i in 0..128 {
                        if least_amount>amount_occurences[i] {
                            least_recurring_char=i;
                            least_amount=amount_occurences[i];
                        }
                    }
                    if least_recurring_char != 128 {
                        let mut find_two_byte_char = |el_to_rem: &str| {
                            loop {
                                if latest_available_2_byte[1]==255 {
                                    latest_available_2_byte = [latest_available_2_byte[0]+1,128]
                                } else {
                                    latest_available_2_byte[1]+=1;
                                }
                                if !taken_2_byte_chars.contains_key(&latest_available_2_byte) {
                                    larger_shortcut.push(latest_available_2_byte);
                                    Rc::clone(&larger_content).borrow_mut().push(el_to_rem.to_string());
                                    replace_everywhere(str::from_utf8(&[least_recurring_char as u8]).unwrap(), str::from_utf8(&latest_available_2_byte[..]).unwrap());
                                    break;
                                }
                            }
                        };
                        if shortcut_eight_byte_chars.contains(&(least_recurring_char as u8)) {
                        let pos = shortcut_eight_byte_chars.iter().position(|n| n == &(least_recurring_char as u8)).unwrap();
                        let el = content_bound_w_eight_bytes.borrow()[pos].clone();

                        if el.len()as u16*least_amount>2*(least_amount+1)+4 {
                            find_two_byte_char(&el);
                        }
                        
                        Rc::clone(&content_bound_w_eight_bytes).borrow_mut()[pos] = mat.to_string();
                        replace_everywhere( mat, str::from_utf8(&[shortcut_eight_byte_chars[pos]]).unwrap());
                        
                    } else if (least_amount+1)*2+4<amount_mat {
                        let mut els = String::new();
                        els = str::from_utf8(&[least_recurring_char as u8]).unwrap().to_string();
                        let el = els.as_str();
                
                        find_two_byte_char(el);
                    }

                    }
                }
            }
        }
    }
        
    //Remove spaces, newlines and comments
    let re = Regex::new(r"<.*>([^<]+)</.*>").unwrap();
    let buffer_ref = buffer.borrow();
    let mut buffer_mut = buffer.borrow_mut();

    for ((full, [text])) in re.captures(&*buffer_ref).map(|c| c.extract()) {
       *buffer_mut = buffer_mut.replace(full, full.replace(text, text.replace(" ", "ķ").replace("\n", "Ĺ").as_str()).as_str());
    }

    let mut quote_char: u8 = 0;
    let mut buf_bytes: Vec<u8> = Vec::new();
    let mut i = 0;
    let mut last_index = 0;
    let mut index_difference = 0;

    let mut current_scope = JsScope::new(None,  false, InsideOf::NormalCode);
    

    let mut inside_for = false;

    let mut current_function = DuoValueFunc{func: [const_fn, const_fn]};

    

    let mut all_values_count: HashMap<NewValueName, u16> = HashMap::new();

    let file = File::open(file_input).await?;


    let (tx, mut rx) = mpsc::channel::<[u8;1024]>(4);


    let not_inside_quote= Rc::new(RefCell::new(false));
    let not_inside_quote_clone = Rc::clone(&not_inside_quote);






    tokio::spawn(async move {
        let mut line_to_send: Vec<u8> = Vec::new();
        let mut buffer  = [u8;1024]([0;2024]);
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 || tx.send(buffer.clone()).await.is_err() {
                break;
            }

        }
    });



    let mut amount_occurence_char_one_byte: [u32;128]= [0;128];
    let mut amount_occurence_char_two_byte: HashMap<[u8;2], u16> = HashMap::new();
    let mut amount_occurence_char_three_byte: HashMap<[u8;3], u16> = HashMap::new();
    let mut amount_occurence_char_four_byte: HashMap<[u8;4], u16> = HashMap::new();

    

    let mut resuming_list: Vec<u8> = Vec::new();  

    
    let mut going_through: GoingThrough = GoingThrough::HTML;
    let was_originally_html = {
        if file_input.ends_with(".html") {
            true
        } 
        else if file_input.ends_with(".css") {
            going_through = GoingThrough::CSS;
            false
        }
        else if file_input.ends_with(".js") {
            going_through = GoingThrough::JS;
            false
        } else {
            false
        }
        
        
    };
    const length: usize = 1024;
    let mut i_buffer: usize = 0;
    let mut previous_line_state = PreviousLineState::Code;

    let mut buffers = {
        Buffers {
            previous_buffer: [0;1024],
            current_buffer: [0;1024],
            buffer_ref: &[0;1024],
            going_to_current: true,
            last_index: 0,
        }
    };
    let mut last_char: u8 = 0;

    while let Some(buffer_bytes) = rx.recv().await {
        buffers.previous_buffer = buffers.current_buffer;

        buffers.current_buffer = buffer_bytes;
        drop(buffers.buffer_ref);
        (buffers.buffer_ref, i_buffer) = if buffers.going_to_current {
            (&buffers.current_buffer, 0)
        } else {
            (&buffers.previous_buffer, buffers.last_index)
        };
        
        let mut i_buffer = if buffers.going_to_current {0} else {};

        let mut continuing = true;

        
        
        while buffers.check_index(&mut i_buffer){


            
           tokio::task::yield_now().await;
            

            let byte = buffers.buffer_ref[i_buffer];
            
            if quote_char!=0 {

                //bad bad bad to change
               find_quote_chars(&buffers.buffer_ref, &mut current_scope, &mut i_buffer, &mut quote_char, &mut previous_line_state, &mut going_through, last_index);
            }


            else {

                

                match last_char {

                    //If the last character was a space and the character before it is a letter AND this character is also a letter, add the space.
                    characters::SPACE => {
                        match buffers.buffer_ref[i_buffer] {
                            
                            32 | 34..=47 | 58..=63 | 91 | 93 | 123..=125 => {}
                            _ => {
                                current_scope.characters.push(characters::SPACE);
                            }
                        }

                    }
                    characters::EQUAL => {
                        match buffers.buffer_ref[i_buffer] {
                            characters::MORE_THAN => {
                                //=>
                            }
                            characters::LEFT_CURLY => {
                                //class declaration
                            }

                        }
                        
                    }

                    


                }
                last_char=0;
                

                buffers.last_index = i_buffer;
                let mut iterated=false;

                loop {

                    match buffers.buffer_ref[i_buffer] {
                        0..=47 | 58..=64 | 91 | 93 | 94 | 123..=128 => {
                            //This ensures that if its a single character, it gets included.
                            if !iterated {
                                i_buffer+=1;
                                continuing = buffers.check_index(&mut i_buffer);
                            
                            }
                            break;
                        }
                        _=> {
                            i_buffer+=1;
                        }
                        

                    }
                    iterated=true;
                    continuing = buffers.check_index(&mut i_buffer);
                    if !continuing {
                        break;
                    }
                }
                if !continuing && iterated {
                    break;
                }

                //If single character 
                if i_buffer == buffers.last_index+1 {
                    let value: u8 = buffers.buffer_ref[buffers.last_index];

                    match value {
                        //If the value is a number, simply add it. No more processing needed.
                        48..58=> {current_scope.characters.push(value)}

                        characters::NEWLINE => {}

                        characters::SPACE => {
                            match previous_line_state {
                                 PreviousLineState::ExpectingScopeChar | PreviousLineState::WaitingParenthesis => {},
                                _ => {
                                    match if i_buffer !=0 {buffers.buffer_ref[i_buffer-1]} 
                                          else if buffers.going_to_current {buffers.previous_buffer[1023]}
                                          else {32} {
                                          32 | 34..=47 | 58..=63 | 91 | 93 | 123..=125 => {}
                                          _ => {
                                            last_char = characters::SPACE;
                                          }

                                    }
                                }
                            }
                            
                        }
                        

                        characters::APOSTROPHE | characters::DOUBLE_QUOTE | characters::GRAVE_ACCENT => {
                            current_scope.characters.push(byte);
                            quote_char = byte;
                        }


                    }
                //If multiple characters 

                } else {
                    let value: Vec<u8> =buffers.buffer_ref[buffers.last_index..i_buffer+1];
                }
               
                match byte {

                    
                    
                    characters::NEWLINE=> {}

                    characters::SPACE=> {
                        match previous_line_state {
                             PreviousLineState::ExpectingScopeChar | PreviousLineState::WaitingParenthesis => {},
                            _ => {
                                if i_buffer != 0 && i_buffer != length-1 {
                                    match buffers.buffer_ref[i_buffer-1] {
                                        32 | 34..=47 | 58..=63 | 91 | 93 | 123..=125 => {}
                                        _ => {
                                            match buffers.buffer_ref[i_buffer+1] {
                                            32 | 34..=47 | 58..=63 | 91 | 93 | 123..=125 => {} 
                                            _ =>{
                                                    current_scope.characters.push(32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                    }

                    characters::APOSTROPHE | characters::DOUBLE_QUOTE | characters::GRAVE_ACCENT => {
                        current_scope.characters.push(byte);
                        quote_char = byte;
                    }
                    
                    //Javascript/CSS Comment
                    characters::SLASH => {
                        if i_buffer+1>length {
                            match buffers.buffer_ref[i_buffer+1] {
                            //Single line comment (like this one)
                            characters::SLASH => {
                                break;
                            } 
                            /*Possibly multi line comment (like this) */
                            characters::ASTERIS => {
                                i_buffer+=1;
                                loop {
                                    i_buffer+=1;
                                    if i_buffer==length {
                                       previous_line_state = PreviousLineState::JSComment;
                                       break;
                                    }
                                
                                    if buffers.buffer_ref[i_buffer] == characters::ASTERIS  {
                                        if buffers.buffer_ref[i_buffer+1] == characters::SLASH {
                                            i_buffer+=1;
                                            break;
                                        }
                                    }
                                }
                            } _ => current_scope.characters.push(characters::ASTERIS)
                        }
                        }
                        
                    }
                    //HTML comment (if not inside concatened string)
                    60 => {
                        if find_if_corresponds(&buffers.buffer_ref, &mut i_buffer, vec![33, 45, 45]) {
                            loop {
                                i_buffer+=1;
                                if i_buffer==length {
                                    previous_line_state = PreviousLineState::HTMLComment;
                                    break;
                                }
                                if buffers.buffer_ref[i_buffer] == 45 && find_if_corresponds(&buffers.buffer_ref, &mut i_buffer, vec![45, 62]) {
                                    break;

                                }
                            }
                        }
                    }
                    characters::RIGHT_CURLY=> {

                        while current_scope.end_scope_char == EndOfScopeChar::SemiColon {
                            scope_end(&mut current_scope, &mut all_values_count).await;
                        }

                        if current_scope.end_scope_char == EndOfScopeChar::Curly {
                            
                            //ANALYSIS 9.1 (See doc for more info)
                            /*Analyze if we can replace curly brackets in a scope */

                            let changing_to_semi_column = current_scope.values.len() as u16 == current_scope.min_value_declarations && (!current_scope.is_a_parent || current_scope.semi_column_indexes.len() == {if last_char_semi_column {0} else {1}});
                            if changing_to_semi_column {
                                current_scope.end_scope_char = EndOfScopeChar::SemiColon;

                                if *current_scope.characters.last().unwrap() == 59 { current_scope.semi_column_indexes.pop(); } else { current_scope.characters.push(59); }

                                //As long as the scope isn't inside an if/else statement, change all the semicolons by commas
                                match current_scope.inside_of {
                                    InsideOf::ElseStatement | InsideOf::IfElseStatement | InsideOf::IfStatement =>{},

                                    _ => {
                                        for index in current_scope.semi_column_indexes {

                                            //TEST (Remove at production)
                                            if current_scope.characters[index] != 59 {
                                                panic!("INDEX OF SEMI COLUMN DOES NOT POINT TO SEMI COLUMN");
                                            }
                                            current_scope.characters[index] = 44;
                                        }
                                    }
                                }
                                
                                
                            } else {
                                current_scope.characters.push(125)
                            }

                            scope_end(&mut current_scope, &mut all_values_count, &mut previous_line_state).await;
                            
                            
                        }

                    }
                    
                    

                    //=>
                   
                    //if
                    105 if find_if_corresponds(&buffers.buffer_ref, &mut i_buffer, vec![102], &mut previous_line_state) => {
                        for inside in [InsideOf::UpperIfStatement(Vec::new(), false), InsideOf::IfStatement] {
                            current_scope = JsScope::new(Some(current_scope), inside);
                        }
                        
                        previous_line_state = PreviousLineState::WaitingParenthesis;
                    }
                    //else
                    101 if match current_scope.inside_of{InsideOf::UpperIfStatement(_,_ )=>true,_=>false} && find_if_corresponds(&buffers.buffer_ref, &mut i_buffer, vec![108, 115, 101], &mut previous_line_state) => {
                        let new_inside_of = if find_if_corresponds(&buffers.buffer_ref, &mut i_buffer, vec![32,105,102], &mut previous_line_state) {
                            InsideOf::IfElseStatement 
                        } else {
                            InsideOf::ElseStatement
                        };
                        current_scope = JsScope::new(Some(current_scope),  new_inside_of);
                    }
                    //function
                    102 if find_if_corresponds(&buffers.buffer_ref, &mut i_buffer, vec![117,110,99,116,105,111,110,32], &mut previous_line_state) {
                        
                        find_function_name(&buffers.buffer_ref, &mut current_scope, &mut i_buffer, &mut all_values_count);
                        while buffers.buffer_ref[i_buffer] != 40 {
                            i_buffer+=1;
                        }
                        current_scope.characters.push(40);
                        
                        new_values_init(&buffers.buffer_ref, &mut current_scope, &mut i_buffer, &mut all_values_count);
                    }



                    
                    //;
                    59 => {
                        while current_scope.end_scope_char == EndOfScopeChar::SemiColon {
                            scope_end(&mut current_scope, &mut all_values_count, &mut previous_line_state);
                        }
                        if current_scope.end_scope_char == EndOfScopeChar::Curly {
                            current_scope.semi_column_indexes.push(current_scope.characters.len());
                            
                        } 
                        current_scope.characters.push(59);
                        
                    }
                    //)
                    41 => {
                        if current_scope.end_scope_char == EndOfScopeChar::Parenthesis {
                            scope_end(&mut current_scope, &mut all_values_count, &mut previous_line_state);
                            current_scope.characters.push(41);

                       
                        } else if let PreviousLineState::WaitingEndScopeChar(x) = &previous_line_state {
                                
                                if *x == 0 {
                                    drop(x);
                                    previous_line_state = PreviousLineState::ExpectingScopeChar;
                                }
                                *x-=1;
                            
                        } else {
                            current_scope.characters.push(41);
                        }
                        
                    }
                    //(
                    40 => {
                        match &previous_line_state {
                            PreviousLineState::WaitingParenthesis => previous_line_state = PreviousLineState::WaitingEndScopeChar(0),
                            PreviousLineState::WaitingEndScopeChar(x) => {
                                *x+=1;
                                current_scope.characters.push(40);
                            },
                            PreviousLineState::ExpectingScopeChar => {
                                assign_end_scope_char(&mut current_scope, &mut previous_line_state, EndOfScopeChar::Parenthesis);
                            }
                            _=>{},
                        }
                    }
                }
            }

            
            
            
            match byte {
                32 | 10 => {},
                _=> {
                    match current_scope.inside_of {
                        InsideOf::UpperIfStatement(_,_)=>{
                            previous_line_state =  PreviousLineState::Code;
                            scope_end(&mut current_scope, &mut all_values_count, &mut previous_line_state);
                        } ,
                        _=> {
                            if previous_line_state == PreviousLineState::ExpectingScopeChar {
                                assign_end_scope_char(&mut current_scope, &mut previous_line_state, EndOfScopeChar::SemiColon);
                            }
                        }
                    }
                }
            }
            i_buffer+=1;
         
        }
        
    }


    

    while i < buf_bytes.len() {
        let byte = buf_bytes[i];
        if quote_char!=0 {
            
       
        } else {
            
            match byte { 
                //function => 0
                //let => 1
                //const => 2
                //var => 3

                10 => {}
                32 => {
                }
                34 | 39 | 96 => {
                    quote_char = byte;
                } 47 => {
                    match buf_bytes[i+1] {
                        47 => {
                            loop {
                                i+=1;
                                if buf_bytes[i] == 10 {
                                    break;
                                }
                            }
                        } 
                        42 => {
                            loop {
                                i+=1;
                                if buf_bytes[last_index] == 42  {
                                    if buf_bytes[last_index+1] == 47 {
                                        i+=1;
                                        break;
                                    }
                                }
                            }
                        } _ => {drain_values(&mut buf_bytes, &mut i, &mut last_index);}
                    }
                }
               
                125 => {
                    
                    if let Some(scope) = current_scope.parent_scope {
                        let all_children_used_values = current_scope.children_used_values.clone();
                        let mut unused_values: HashMap<u8, Option<usize>> = HashMap::new();

                        for val in all_values_count.keys().cloned() {
                            if let NewValueName::OneChar(one_char) = val {
                                
                                let mut unused=true;
                                for value in &current_scope.values {
                                    if value.value_new_name == val {
                                        unused=false;
                                        break;
                                    }
                
                                }
                                
                                if unused {
                                    let mut present_after: bool = false;
                                    for child_value in current_scope.children_used_values.clone() {
                                        if child_value.value_new_name == val {
                                            unused_values.insert(one_char, Some(child_value.last_usage_index));
                                            present_after=true;
                                            break;
                                        }
                                    }
                                    if !present_after {
                                        unused_values.insert(one_char, None);
                                    }
                                    
                                }
                            }
                            
                        }
                        let mut changed_values: HashMap<[u8;2], u8> = HashMap::new();
                        
                        for value in &mut current_scope.values {
                            if let NewValueName::TwoChar(two_char) =value.value_new_name {
                            let mut changed_value : Option<u8> = None;
                            
                            for (new_val, index) in &unused_values {
                                /*If the last time the value was called is before the assignment
                                 of the value we want to replace, then we can go ahead and replace it
                                 without affecting the functionnality of the code. */
                                if match index {
                                   Some(i)=>{*i<value.declaration_index }
                                   None=>{true}
                                }{
                                    changed_values.insert(two_char, *new_val);
                                    changed_value = Some(*new_val);
                                    value.value_new_name = NewValueName::OneChar(*new_val);
                                    break;
                                }
                                    
                                }
                                if let Some(v) = changed_value {
                                    unused_values.remove(&v);
                                }
                            }
                        }

                        let mut j = current_scope.starting_index;
                        let mut q_char: u8 = 0;
                        loop {
                            {
                                let len = current_scope.characters.len();
                                if j >= len {
                                    break;
                                }
                            }
                            // Now only mutable borrow
                            find_value_to_replace(&mut q_char, &mut j, &changed_values, current_scope);
                            tokio::task::yield_now();
                        }
                        let parent = scope.borrow();
                        
                        current_scope = parent.clone();
                        current_scope.children_used_values = all_children_used_values;
                            
                     }
                    }
                
                123 => {
                    if !inside_for {
                        current_scope = JsScope::new(Some(current_scope), i, false);
                    } else {
                        inside_for = false;
                    }
                }
                //=>
                61 => {
                    drain_values(&mut buf_bytes, &mut i, &mut last_index);

                    if buf_bytes[i+1] == 62 {
                        let mut j =i;
                        current_scope = JsScope::new(Some(current_scope), i,  false);
                        
                        while buf_bytes[j] != 40 && buf_bytes[j] != 61 {
                            j-=1;
                        }
                        let mut param_name: Vec<u8> = Vec::new();
                        loop {
                            match buf_bytes[j] {
                                28 => {
                                    add_value(&mut current_scope, &mut all_values_count,  std::mem::take(&mut param_name), i);
                                    
                                } 
                                61 | 41 => {
                                    add_value(&mut current_scope, &mut all_values_count, param_name, i);
                                    break;
                                } 
                                _ => {
                                    param_name.push(buf_bytes[j]);
                                }
                            }
                        }
                        i+=1;
                        last_index+=1;
                        let mut to_empty=false;
                        while buf_bytes[i+1] == 32 || buf_bytes[i+1] == 10 {
                            i+=1;
                            to_empty=true;
                        }
                        if to_empty {
                            buf_bytes.drain(last_index+1..i);
                            i=last_index;
                        }
                        if buf_bytes[i+1] == 123 {
                            i+=1;
                            last_index+=1;
                        }
                    }
                }
                 _ => {

                    drain_values(&mut buf_bytes, &mut i, &mut last_index);
                    //const 
                    if byte == 99 && buf_bytes[i+1] == 110 && buf_bytes[i+2] == 111 && buf_bytes[i+3] == 115 && buf_bytes[i+4] == 116 && buf_bytes[i+5] == 32 {
                        last_index+=5;
                        i+=5;
                        let mut end_index=i;
                        let mut value_name: Vec<u8> = Vec::new();

                        const_fn(&mut FnParams {
                                i: &mut i,
                                end_index: &mut end_index,
                                buf_bytes: &mut buf_bytes,
                                value_name: &mut value_name,
                                all_values_count: &mut all_values_count,
                                last_index: &mut last_index,
                                current_scope: &mut current_scope,
                        });
                    }
                    //function
                    else if byte == 102 && buf_bytes[i+1] == 117 && buf_bytes[i+2] == 111 && buf_bytes[i+3] == 99 && buf_bytes[i+4] == 116 && buf_bytes[i+5] == 105 && buf_bytes[i+6] == 111 && buf_bytes[i+7] == 110 && buf_bytes[i+8] == 32 {
                        i+=8;
                        last_index+=8;
                        let mut end_index=i;
                        let mut function_name: Vec<u8> = Vec::new();

                        loop {
                            end_index+=1;
                            match buf_bytes[end_index] {
                                40 => {
                                    add_value(&mut current_scope, &mut all_values_count, function_name, i);
                                    break;
                                    
                                } _ => {
                                    function_name.push(buf_bytes[i]);
                                }
                            }
                        }

                        

                    }
                    //let
                    else if byte == 108 && buf_bytes[i+1] == 101 && buf_bytes[i+2] == 116 && buf_bytes[i+3] == 32 {
                        i+=3;
                        last_index+=3;
                        let mut end_index = i;
                        let mut value_name: Vec<u8> = Vec::new();



                        const_fn(&mut FnParams {i: &mut i,
                                                end_index: &mut end_index,
                                                buf_bytes: &mut buf_bytes,
                                                value_name: &mut value_name,
                                                all_values_count: &mut all_values_count,
                                                last_index: &mut last_index,
                                                current_scope: &mut current_scope});
                        
                    }
                    //var
                    else if byte == 118 && buf_bytes[i+1] == 97 && buf_bytes[i+2] == 114 && buf_bytes[i+3] == 32 {

                    }
                    //for
                    else if byte == 102 && buf_bytes[i+1] == 111 && buf_bytes[i+2] == 114 {
                        i+=3;
                        last_index=i-1;
                        let mut did = false;
                        while buf_bytes[i] == 32 {
                            i+=1;
                            did=true;
                        }
                        if did {
                            buf_bytes.drain(last_index+1..i);
                            i=last_index;
                        }
                        if buf_bytes[i+1] == 40 
                        {
                            current_scope = JsScope::new(Some(current_scope),  false, InsideOf::ForLoop);
                            inside_for= true;
                            i+=1;
                        }
                    } else {
                       find_value(&mut buf_bytes, &mut current_scope, &mut i, &mut last_index, &mut all_values_count);
                    }

                 }

            }
        }
        i+=1;
    }

    *buffer_mut = buffer_mut.replace("ķ", " ").replace("Ĺ", "\n");
    Ok(())
}

fn ajouter(a: i32, b: i32 )-> i32 {
    return a+b;
}
fn multiplier(a: i32,b: i32)-> i32 {
    return a*b;
}
fn soustraire(a: i32, b:i32)-> i32 {
    return a-b;
}



pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    



    use super::*;

    #[test]
    fn test() {
        //Array statique de fonctions 
        let liste_de_fonctions: [fn (i32,i32) -> i32; 3] = [ajouter, multiplier, soustraire];
        let mut i =0;
        while i < 3 {
            let fonction_a_executer: fn(i32, i32) -> i32 = liste_de_fonctions[i];
            let resultat = fonction_a_executer(4, 5);
            println!("Le résultat est {}", resultat);
            i+=1;
        }
        
    }
}


//TODO
//Add something that gets all the used values from a function when its called.

/*Current issue, lets consider this code:

    let value1 = 5;

    function myFunc() {
        console.log(value1);
    }

    let value2 = 2;

    myFunc();


With the current code's logic, value2 could have the same one char value name as value1, but because we then call
a function using value1, its gonna be replaced by value2 which is not what we want.

We need a data structure to track all of the used values of functions that could potentially be called
in order to update the "last_usage_index" property of each value used inside the function each time the function gets called.

*/