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

    pub fn reserve(&mut self, size: usize) -> &mut [u8]{
        unsafe{
            let mut pointer = self.write_pointer.load(Ordering::Acquire);
            let alignment = core::mem::align_of::<usize>();
            let modulo = (pointer as usize ) % alignment; 
            if modulo != 0 {
                pointer = pointer.add(alignment - ( (pointer as usize ) % alignment)); // Align pointer to usize alignment
            }
            core::ptr::copy_nonoverlapping(
                &size as *const usize,
                pointer as *mut usize,
                1
            );
            pointer = pointer.add(size_of::<usize>()); //set pointer after inserted usize
            core::slice::from_raw_parts_mut::<u8>(pointer, size)
        }
        
    }

    pub fn declare(&mut self){
        unsafe{
            let pointer = self.write_pointer.load(Ordering::Acquire);
            let alignment = core::mem::align_of::<usize>();
            let modulo = (pointer as usize ) % alignment; 
            let difference = if modulo == 0 {0} else {alignment - ( (pointer as usize ) % alignment)}; // Align pointer to usize alignment
            let size = *(pointer.add(difference) as *const usize);
            println!("{}", size);
            self.write_pointer.store(pointer.add(difference+core::mem::size_of::<usize>()+size), Ordering::Release);
            println!("{:#018x}", self.write_pointer.load(Ordering::Acquire) as usize);
        }
    }
}


fn main() {
    println!("Hello, world!");
    let mut buff = CircullarBuffer::new();

    let a = buff.reserve(3);
    a[0] = 4;
    a[1] = 15;
    a[2] = 100;
    let addr = &buff.data as *const _ as usize; 
    for i in 0..30{
        println!("({}){:#018x}: {}", i, addr+i, buff.data[i]);
    }
    println!("SECOND ROUND");
    buff.declare();
    let b = buff.reserve(2);
    b[0] = 69;
    b[1] = 88;

    let addr = &buff.data as *const _ as usize; 
    for i in 0..30{
        println!("({}){:#018x}: {}", i, addr+i, buff.data[i]);
    }
}

