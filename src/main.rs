use core::mem::size_of;
use core::sync::atomic::{AtomicPtr, Ordering};

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
    data: [u8; BUFFER_SIZE],
    read_pointer: AtomicPtr<u8>,
    write_pointer: AtomicPtr<u8>,
    release_pointer: AtomicPtr<u8>,
}


impl CircullarBuffer{
    pub fn new() -> Self{
        let mut buff = CircullarBuffer{
            data: [0;BUFFER_SIZE],
            read_pointer: AtomicPtr::new(&mut 0),
            write_pointer: AtomicPtr::new(&mut 0),
            release_pointer: AtomicPtr::new(&mut 0),
        };
        
        let address = &mut buff as *mut _ as *mut u8;
        buff.read_pointer.store(address, Ordering::Relaxed);
        buff.write_pointer.store(address, Ordering::Relaxed);
        buff.release_pointer.store(address, Ordering::Relaxed);

        println!("{:#018x}", address as u64);

        buff
    }

    // fn add_val<T>(&mut self, value: T, entry_type: u64){
    //     unsafe{
    //         let preamble = BufferEntryPreamble{
    //             entry_type: entry_type,
    //             size: size_of::<T>() as u64,
    //         };
            
    //         unsafe {
    //             let ptr = 
    //             core::ptr::copy_nonoverlapping(
    //                 &preamble as *const BufferEntryPreamble,
    //                 self.write_pointer.load(Ordering::Acquire) as *mut BufferEntryPreamble,
    //                 1,
    //             );

    //             self.write_pointer. += size_of::<BufferEntryPreamble>() as u64;

    //             core::ptr::copy_nonoverlapping(
    //                 &value as *const T,
    //                 self.write_pointer as *mut T,
    //                 1,
    //             );
                
    //             self.write_pointer += size_of::<T>() as u64;

    //         }
    //     }
    // }

    // fn read_value<T>(&mut self) -> T{
    //     let mut res: T;

    //     unsafe{
    //         self.read_pointer += size_of::<BufferEntryPreamble>() as u64;
    //         core::ptr::copy_nonoverlapping(
    //             self.read_pointer as *const T,
    //             &mut res as *mut T,
    //             1
    //         );
    //     }

    //     res

    // }

    pub fn add_value(&mut self, data: &[u8]){
        unsafe{
            let mut pointer = self.write_pointer.load(Ordering::Acquire);
            core::ptr::copy_nonoverlapping(
                &data.len() as *const _ as *const u64,
                pointer as *mut u64,
                1
            );
            pointer = pointer.add(8); //add u64
            core::ptr::copy_nonoverlapping(
                data as *const _ as *const u8,
                pointer,
                data.len()
            );
            pointer = pointer.add(data.len());
            self.write_pointer.store(pointer, Ordering::Release);
        }
    }
}


fn main() {
    println!("Hello, world!");
    let mut buff = CircullarBuffer::new();

    buff.add_value(&[69]);
    buff.add_value(&[21, 37]);
    buff.add_value(&[23, 12, 32, 12]);

    for i in 0..30{
        println!("{}: {}", i, buff.data[i]);
    }
}

