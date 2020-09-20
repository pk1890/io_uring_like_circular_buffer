use core::mem::size_of;

const BUFFER_SIZE: usize = 1024;

// #[repr(u64)]
enum BufferEntry{
    Simple(u64),
    Double([u64;2])
}

#[repr(C)]
struct BufferEntryPreamble{
    entry_type: u64,
    size: u64
}

#[repr(C)]
struct CircullarBuffer{
    data: [u64; BUFFER_SIZE],
    read_pointer: u64,
    write_pointer: u64,
    release_pointer: u64,
}


impl CircullarBuffer{
    pub fn new() -> Self{
        let mut buff = CircullarBuffer{
            data: [0;BUFFER_SIZE],
            read_pointer: 0,
            write_pointer: 0,
            release_pointer: 0
        };
        
        let address = &buff as *const _ as u64;
        buff.read_pointer = address;
        buff.write_pointer = address;
        buff.release_pointer = address;

        println!("{:#018x}", address);

        buff
    }

    fn add_val<T>(&mut self, value: T, entry_type: u64){
        unsafe{
            let preamble = BufferEntryPreamble{
                entry_type: entry_type,
                size: size_of::<T>() as u64,
            };
            
            unsafe {
                core::ptr::copy_nonoverlapping(
                    &preamble as *const BufferEntryPreamble,
                    self.write_pointer as *mut _,
                    1,
                );

                self.write_pointer += size_of::<BufferEntryPreamble>() as u64;

                core::ptr::copy_nonoverlapping(
                    &value as *const _,
                    self.write_pointer as *mut _,
                    1,
                );
                
                self.write_pointer += size_of::<T>() as u64;

            }
        }
    }

    // fn read_value<T>(&mut self)

    pub fn add_value(&mut self, entry: BufferEntry){
        match entry{
            BufferEntry::Simple(val) => self.add_val(val, 0),
            BufferEntry::Double(val) => self.add_val(val, 1),
        }
    }
}


fn main() {
    println!("Hello, world!");
    let mut buff = CircullarBuffer::new();

    buff.add_value(BufferEntry::Simple(420));
    buff.add_value(BufferEntry::Double([21, 37]));
    buff.add_value(BufferEntry::Simple(69));

    for i in 0..10{
        println!("{}", buff.data[i]);
    }
}

