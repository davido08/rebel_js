/*This file contains all of the structures and pub enums necessary in order to analyse the
 HTML/CSS/Javascript code in order to shorten syntax/value names as much possible. */
use crate::characters;
use crate::compression;
use std::cell::RefCell;
use std::rc::Rc;



///This is a more efficient version of a HashMap, with lookup speed of approximately O(1), 
///less memory usage as well as faster index calculation than a Hash.
/// 
/// It usees a combination of the first byte 
/// and the two last bits of the middle byte as an index inside the table
pub struct ValueList ([Vec<Rc<RefCell<Value>>>; 1024]);



impl ValueList {
    pub fn new() -> Self {
        return ValueList (std::array::from_fn(|_| Vec::new()));
    }
    
    ///Insert a new value in the table, returns wether or not a value with the same old name is already inside the list
    pub fn insert(&mut self, value: Rc<RefCell<Value>>, index: usize) {
        self.0[index].push(value);
    }
    ///Runs when a value is being accessed, returns the new name of the value if found.
    pub fn update(&mut self, old_value: Vec<u8>, position: usize, index: usize) -> Option<NewValueName> {

        let value_to_search = old_value[1..];
        //We are iterating reversely in the list because if there are two values with the same old name, the one added more recently inside the list will be the one to access.
        for val in self.0[index].iter().rev() {
           
            if val.borrow().value_old_name == value_to_search {
                val.borrow_mut().amount_occurences+=1;
                val.borrow_mut().last_usage_index = position;
                
                return Some(val.borrow().value_new_name);
            }
        }
        return None;

    }

}
   
pub struct ChildValuesList {
    pub list: ValueList,
    pub new_value_last_position: [Option<usize>; 52]
}

impl ChildValuesList {

    fn update_new_value_last_position(&mut self, value: Rc<RefCell<Value>>) {
        if let NewValueName::OneChar(val) = value.borrow().value_new_name {
           let index = match val {
                //Uppercase letters (Index: 0..=25)
                65..=90 => val-65,
                //Lowercase letters (97..=122) (Index: 26..=51)
                _ => val-71
            } as usize;

            self.new_value_last_position[index] = value.borrow().last_usage_index;
        }
    }

    pub fn insert(&mut self, value: Rc<RefCell<Value>>, index: u8) {
        self.list.insert(value, index);
        self.update_new_value_last_position(value);

    }
    


    pub fn update(&mut self, value: Vec<u8>, position: usize, index: usize)  {
        if let Some(name) = self.list.update(value, position, index) {
            self.update_new_value_last_position(value);
            
        }
    }

    pub fn consume(&mut self, list_to_consume: ValueList) {
        for (index, bucket) in  list_to_consume.0.iter().enumerate() {
            while let Some(value) = bucket.pop() {
                self.list.insert(value, index);

            }
        }
    }
}


pub struct BitSet64 {
    pub bitset: u64,
    pub offset: u8,
}

impl BitSet64 {
    ///Set a specific bit inside the BitSet to 1 (true)
    pub fn insert(&mut self, position: u8) {
        self.bitset = self.bitset | (1 << (position - self.offset));
    }
    ///Set a specific bit inside the BitSet to 0 (false)
    pub fn remove(&mut self, position: u8) {
        self.bitset = self.bitset & !(1 << (position - self.offset));
    }
    ///Get the value of a specific bit
    pub fn get(&self, position: u8) -> bool {
        return 1 & (self.bitset >> (position - self.offset)) == 1;
    }
}
pub struct CodeAnalysisObject<'a> {
    pub previous_buffer: [u8;1024],
    pub current_buffer: [u8;1024],
    pub buffer_ref: &'a [u8;1024],
    pub going_to_current: bool,
    pub last_index: usize,
    pub i_buffer: usize, 
    pub last_char: u8,
    pub current_scope: JsScope,
    pub character_amounts: compression::CharacterAmounts,
    pub current_char_bytes_remaining: u8,
    pub current_big_char: [u8;4],
    pub previous_buffer_state: PreviousBufferState,
}

impl<'a> CodeAnalysisObject<'a> {

    ///Verifies if it can keep iterating through the current buffer, or if it has to get a new buffer from the IO async task.
    pub fn check_index(&mut self)-> bool {
        if self.i_buffer == 1024 {
            self.going_to_current = !self.going_to_current;
            if self.going_to_current {
                self.buffer_ref = &self.current_buffer;
                self.i_buffer=0;
            }
            return self.going_to_current;
        }
        return true;
    }
    
    pub fn add_one_byte_char(&mut self, byte: u8) {
        self.current_scope.push(byte);
        self.character_amounts.add_small_char(byte);
    }
    
    pub fn add_byte_for_big_char(&mut self, byte: u8) {
        self.current_scope.push(byte);
        if self.current_char_bytes_remaining == 0 {
            self.current_char_bytes_remaining = match byte {
                //2 bytes
                194..=223 => 1,
                //3 bytes
                224..=239 => 2,
                //4 bytes 
                240..=247 => 3,
                _ => {
                    panic!("Incorrect value range");
                }
            };
            self.current_big_char[0] = byte;
            
        } else {
            self.current_char_bytes_remaining-=1;
             
            let mut i: usize = 1;
            while self.current_big_char[i] != 0 {i+=1}
            self.current_big_char[i] = byte;

            if self.current_char_bytes_remaining == 0 {
                self.character_amounts.add_big_char(self.current_big_char);
                self.current_big_char = [0;4];
            }
            
        }

    }

    pub fn get_char(&self) -> u8 {
        return self.buffer_ref[self.i_buffer];
    }


    pub fn go_through_js_comment(&mut self) {
        loop {
            if self.i_buffer+1 == 1024 {
                self.previous_buffer_state = PreviousBufferState::JSComment;
                self.last_index=1024;
                break;
            }
            let byte = self.buffer_ref[self.i_buffer+1];
            match byte {
                characters::SLASH => {
                    if self.last_char == characters::ASTERIS {
                        break;
                    }
                }
               
            }
            self.last_char = byte;
        }

    }


    pub fn go_through_js_inline_comment(&mut self) {
        loop {
            self.i_buffer;
            if self.i_buffer+1==1024 {
                self.previous_buffer_state = PreviousBufferState::JSInlineComment;
                self.last_index=1024;
                break;
            }
            if self.buffer_ref[self.i_buffer+1] == characters::NEWLINE {
                break;
            }
            self.i_buffer+=1;

        }
    }
    pub fn verify_if_else_statement(&mut self, mut i_found: bool) {
        let mut else_if_statement = false;
        loop {
            self.i_buffer+=1;
            if self.i_buffer==1024 {
                self.last_index=1024;
                self.previous_buffer_state = if i_found {
                    PreviousBufferState::WaitingIfStatementF
                } else {
                    PreviousBufferState::WaitingIfStatementFirstI
                };
                break;
            }
            match self.get_char()  {
                characters::SPACE => {
                    if i_found {
                        break;
                    }
                }
                b'i' => {
                    if !i_found {
                        i_found = true;
                    } else {
                        break;
                    }
                    
                }
                b'f' => {
                    else_if_statement = i_found;
                    break;
                }
            }
            
        }
        if self.i_buffer!=1024 {
            let new_inside_of = if else_if_statement {
                InsideOf::IfElseStatement 
            } else {
                InsideOf::ElseStatement
            };
            self.current_scope = JsScope::new(Some(self.current_scope),  new_inside_of);
            
        }
    }

    pub fn find_function_name(&mut self){
        let mut function_name: Vec<u8> = Vec::new();
        let value = {
            loop {
                self.i_buffer+=1;
                if !self.check_index() {
                    self.previous_buffer_state = PreviousBufferState::FindingFunctionName;
                    return;
                }
                
                match *self.buffer_ref[self.i_buffer] {
                    32 | 40 => break,
                    x=> function_name.push(x)
                }
            }
            self.current_scope.new_value(function_name);
        
        };
    
        while self.buffer_ref[self.i_buffer]!=40{self.i_buffer+=1;}
    
        let mut function_param: Vec<u8> = Vec::new();
        self.current_scope = JsScope::new(Some(self.current_scope),   InsideOf::Function(value.value_new_name));
    }

    pub fn new_values_init(&mut self) {
        

        let mut param_name: Vec<u8> = Vec::new();
        let mut new_values: Vec<NewValueName>;
        loop {
            if !self.check_index() {
                self.previous_buffer_state = PreviousBufferState::FindingNewValueName;
                return;
            }

            match *self.buffer_ref[self.i_buffer] {
                32 | 10 => {},
                40 | 61 => {
                    
                    new_values.push(self.current_scope.new_value( std::mem::take(&mut param_name)));
                    break;
                }
                44 => new_values.push(self.current_scope.new_value( std::mem::take(&mut param_name))),
                x=>param_name.push(x),
            }
            self.i_buffer+=1;
        }
       
        if new_values.len()>1 {
            self.add_one_byte_char(40);
            for value_name in new_values {
                match value_name {
                    NewValueName::OneChar(x) =>self.add_one_byte_char(x),
                    NewValueName::TwoChar(x) => {
                        for char in x {
                            self.add_one_byte_char(char)
                        }
                    }
                }
                self.add_one_byte_char(44);
            }
            *self.current_scope.characters.last_mut().unwrap() = characters::RIGHT_PARENTHESIS;
        }
        else {
            match new_values[0] {
                NewValueName::OneChar(x) => self.add_one_byte_char(x),
                NewValueName::TwoChar(x) => {
                    for char in x {
                        self.add_one_byte_char(char);
                    }
                }
            }
            
        }
    
    }
    
}

pub struct PotentialValue {
    pub name: Vec<u8>,
    pub is_a_function:bool,
    pub blocked_values: BitSet64,

}

impl PotentialValue {
    pub fn new(name: Vec<u8>, is_a_function: bool, blocked_values_vec: Vec<NewValueName>) -> Self {
        let mut blocked_values: BitSet64 = BitSet64 { bitset: 0, offset: 65 };
        for value in blocked_values_vec {
            match value {
                NewValueName::OneChar(x) => blocked_values.insert(x),
                _=>{}
            }
        }
        PotentialValue {
            name,
            is_a_function,
            blocked_values: blocked_values,
        }
    }
}
pub enum NewValueName {
    OneChar(u8), 
    TwoChar([u8;2]),
}

#[derive(Clone, PartialEq)]
pub enum EndOfScopeChar {
    Curly,
    Parenthesis,
    SemiColon,
    None,
}

#[derive(PartialEq)]
pub enum PreviousBufferState {
    Code, 
    JSInlineComment,
    JSComment,
    HTMLComment, 
    StringConcat([u8;2]),
    WaitingParenthesis,
    WaitingEndScopeChar(u8),
    ExpectingScopeChar,

    //Different states of verifying if its an if statement
    WaitingIfStatementFirstI,
    WaitingIfStatementF,


    FindingNewValueName,
    FindingFunctionName,
}
#[derive(Clone, PartialEq)]
pub enum InsideOf {
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
pub enum GoingThrough {
    HTML,
    CSS, 
    JS
}


#[derive(Clone, PartialEq, Eq)]
pub struct Value {
    ///The old name of the value excluding the first byte (serves as table index)
    pub value_old_name: Vec<u8>,
    pub value_new_name: NewValueName,
    pub amount_occurences: u16,
    pub declaration_index: usize,
    pub last_usage_index: usize,
    pub function_index: Option<usize>,

}

pub struct JsScope {
    //Has to change from original vector to a set like for more efficient access
    pub values: ValueList,
    pub two_byte_name_values: Vec<Value>,
    //Has to change from original vector to a set like for more efficient access
    pub used_values: ValueList,
    pub parent_scope: Option<Rc<RefCell<JsScope>>>,
    pub starting_index: usize,
    pub latest_available_one_byte: u8,
    pub children_used_values: ChildValuesList
    pub characters: Vec<u8>,
    pub inside_of: InsideOf,
    pub end_scope_char: EndOfScopeChar,
    pub min_value_declarations: u16,
    pub all_value_declarations: u16,
    pub semi_column_indexes: Vec<usize>,
    pub is_a_parent: bool,
    pub beginning_chars: Vec<u8>,
    pub all_parent_values: Vec<u8>,
    //Has to change from original vector to a set like for more efficient access
    pub potential_values_called: Vec<PotentialValue>,
    //Has to change from original vector to a set like for more efficient access
    pub potential_values_declared: Vec<(NewValueName,Vec<PotentialValue> )>,
    pub inside_function: Option<NewValueName>,
    
}
impl JsScope {

     pub fn new(parent: Option<JsScope>, inside_of: InsideOf) -> Self {
        let (one_byte, current_index): (u8, usize,) = 
        if let Some(scope) = &parent {
            (scope.latest_available_one_byte, scope.starting_index+scope.characters.len())
        } else {(65, 0)};
        JsScope {
            values: ValueList::new(),
            used_values: ChildValuesList{
                list: ValueList::new(),
                new_value_last_position: std::array::from_fn(|_| None)
            },
            all_parent_values: ValueList,
            
            two_byte_name_values: Vec::new(),
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
            potential_values_declared: Vec::new(),
            inside_function: match inside_of { 
                InsideOf::Function(x) => Some(x),
                _=>  
                 match parent {
                    Some(p) => p.inside_function,
                    None=>None
                }
            },


        }
    }
     //Getting the index of the bucket where the value will be located on the ValueList table in order to access it.
     pub fn get_value_index(&mut self, old_name: Vec<u8>) -> usize {
        /*First byte: 0b'01XXXXXX' since a one byte character must be at least as high as a letter, 
        and if its a multi byte character (first byte is 0b'11XXXXXX'), the second character will be 0b'1XXXXXXX'
        so there are 0 risks of collision.*/

        //Take the two lasts bits of the last byte (Z) and two last bits of the middle byte (Y) : 0b'ZZYYXXXXXX'

        return (old_name[0] as usize | 0b1111000000) & ((old_name[old_name.len() >> 1]  << 6) as usize | 0b1100111111) & ((old_name.last().unwrap() << 8) as usize | 0b0011111111);
     }

     pub fn new_value(&mut self, old_name: Vec<u8>) -> (Value, u8) {

        //Find if the declared value has been used before in the code (Like in a function that hasn't been called before value declaration)
        let mut i=0;

        while i < self.potential_values_declared.len() {
            let potential_value = self.potential_values[i];
            if potential_value.name == old_name {

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
         /*First byte: 0b'01XXXXXX' since a one byte character must be at least as high as a letter, 
        and if its a multi byte character (first byte is 0b'11XXXXXX'), the second character will be 0b'1XXXXXXX'
        so there are 0 risks of collision.*/

        //Take the last bit of the last byte (Z) and last bit of the middle byte (Y) : 0b'ZYXXXXXX'

        let index = (old_name[0] | 0b11000000) & ((old_name[old_name.len() >> 1] << 6) | 0b10111111) & ((old_name.last().unwrap() << 7) | 0b01111111);
        let position = self.characters.len()+ self.starting_index;

       let new_value: Rc<RefCell<Value>> = Rc::new(RefCell::new(Value {
            value_old_name: old_name[1..].to_vec(),
            value_new_name : new_name,
            amount_occurences: 0,
            /*Because the index is the length of the current_scope, to know if a value has been used before in a child scope, 
            we'll have to check the postion of the scope itself*/
            declaration_index: position,
            last_usage_index: position,
            function_index: if self.is_function {
                Some(self.starting_index)
            } else {
                None
            },

        }));
        self.values.insert(new_value.clone(), index);
        
        if let NewValueName::TwoChar(_) = new_value.borrow().value_new_name {
            self.two_byte_name_values.push(new_value.clone());
        }

        
        return new_value;
        
    }
    ///Returns the new value name of the value if it is found in either values of the scope or parent values.
    /// 
    /// If it is not found in the scope own values but found in parent values and still not added to thre used values, value will get added.
    pub fn update_value(&mut self, old_name: Vec<u8>) -> Option<NewValueName> {
        let index = self.get_value_index(old_name);
        let position = self.starting_index+self.characters.len();
        if let Some(value) = self.values.update(old_name, position, index)  {
            return Some(value)
        } else if let Some(value) = self.all_parent_values.update(old_name, position, index) {

        }
    }
    

    

}





