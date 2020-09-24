use core::mem::size_of;
use core::sync::atomic::{AtomicPtr, Ordering};

const BUFFER_SIZE: usize = 30;

#[derive(Debug)]
enum BufferAddValueError{
    SizeTooBig
}

#[derive(Debug)]
enum BufferGetValueError{
    NoValueInBuffer
}

#[repr(C)]
struct ReservedMemory<'a>{
    memory: &'a mut [u8],
    buffer: &'a CircullarBuffer,
}

impl<'a> core::ops::Deref for ReservedMemory<'a>{
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &(*self.memory)
    }
}


impl<'a> core::ops::DerefMut for ReservedMemory<'a>{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut(*self.memory)
    }
}

impl<'a> Drop for ReservedMemory<'a>{
    fn drop(&mut self) {
        self.buffer.declare();
    }
}


#[repr(C)]
struct ReturnedValue<'a>{
    memory: &'a [u8],
    buffer: &'a CircullarBuffer,
}

impl<'a> core::ops::Deref for ReturnedValue<'a>{
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &(*self.memory)
    }
}

#[repr(C)]
struct CircullarBuffer{
    data: [u8; BUFFER_SIZE],
    read_pointer: AtomicPtr<u8>,
    write_pointer: AtomicPtr<u8>,
    release_pointer: AtomicPtr<u8>,
    reservation_pointer: AtomicPtr<u8>,
}


impl CircullarBuffer{
    pub fn new() -> Self{
        let mut buff = CircullarBuffer{
            data: [0;BUFFER_SIZE],
            reservation_pointer: AtomicPtr::new(&mut 0),
            write_pointer: AtomicPtr::new(&mut 0),
            read_pointer: AtomicPtr::new(&mut 0),
            release_pointer: AtomicPtr::new(&mut 0),
        };
        
        let address = &mut buff as *mut _ as *mut u8;
        let alignment = core::mem::align_of::<usize>();
        let modulo = (address as usize ) % alignment; 
        let offset = if modulo != 0 {
            alignment - ( (address as usize ) % alignment) // Align pointer to usize alignment
        } else {0};
        let start_address = (address as usize + offset) as *mut u8;
        buff.reservation_pointer.store(start_address, Ordering::Relaxed);
        buff.write_pointer.store(start_address, Ordering::Relaxed);
        buff.read_pointer.store(start_address, Ordering::Relaxed);
        buff.release_pointer.store(start_address, Ordering::Relaxed);

        println!("{:#018x}", address as u64);

        buff
    }


    pub fn reserve(&self, size: usize) -> Result<ReservedMemory, BufferAddValueError>{
        if size >> core::mem::size_of::<usize>()*8-1 != 0 {
            return Err(BufferAddValueError::SizeTooBig);
        }
        unsafe{
            let mut pointer = self.reservation_pointer.load(Ordering::Acquire);
            let alignment = core::mem::align_of::<usize>();
            let modulo = (pointer as usize ) % alignment; 
            if modulo != 0 {
                pointer = pointer.add(alignment - ( (pointer as usize ) % alignment)); // Align pointer to usize alignment
            }

            let end_of_buffer = self as *const _ as usize + BUFFER_SIZE;
            if pointer as usize > end_of_buffer || end_of_buffer - (pointer as usize) - core::mem::size_of::<usize>() < size{
                return Err(BufferAddValueError::SizeTooBig);
            }

            core::ptr::copy_nonoverlapping(
                &size as *const usize,
                pointer as *mut usize,
                1
            );
            pointer = pointer.add(size_of::<usize>()); //set pointer after inserted usize
            let res = ReservedMemory{
                memory: core::slice::from_raw_parts_mut::<u8>(pointer, size),
                buffer: self,
            };
            self.reservation_pointer.store(pointer.add(size), Ordering::Release);
            Ok(res)
        }
    }

    fn declare(&self){
        if self.reservation_pointer.load(Ordering::Acquire) as u64 == self.write_pointer.load(Ordering::Acquire) as u64{
            return;
        }
        let only_msb_of_usize = 1 << core::mem::size_of::<usize>()*8-1;
        let mut pointer = self.write_pointer.load(Ordering::Acquire);
        let alignment = core::mem::align_of::<usize>();
        let modulo = (pointer as usize ) % alignment; 
        let difference = if modulo == 0 {0} else {alignment - ( (pointer as usize ) % alignment)}; // Align pointer to usize alignment
        unsafe{
            let size = &mut (*((pointer.add(difference) as *mut usize)));
            let size_value = *size & !only_msb_of_usize;

            pointer = pointer.add(difference+core::mem::size_of::<usize>()+size_value);
            *size |= only_msb_of_usize;
            loop{
                let size = *(pointer.add(difference) as *const usize) & !only_msb_of_usize;
                let difference = if modulo == 0 {0} else {alignment - ( (pointer as usize ) % alignment)}; // Align pointer to usize alignment
                
                if size >> core::mem::size_of::<usize>()*8-1 == 0{
                    self.write_pointer.store(pointer, Ordering::Release);
                    return;
                }
                pointer = pointer.add(difference+core::mem::size_of::<usize>()+size);
            }

        }
    }

    pub fn get_value(&self) -> Result<ReturnedValue, BufferGetValueError>{
        let mut read_pointer = self.read_pointer.load(Ordering::Acquire);
        let write_pointer = self.write_pointer.load(Ordering::Acquire);
        if read_pointer as usize == write_pointer as usize{
            return Err(BufferGetValueError::NoValueInBuffer);
        }

        let alignment = core::mem::align_of::<usize>();
        let modulo = (read_pointer as usize ) % alignment; 
        let only_msb_of_usize = 1 << core::mem::size_of::<usize>()*8-1;
        unsafe{
            if modulo != 0 {
                read_pointer = read_pointer.add(alignment - ( (read_pointer as usize ) % alignment)); // Align pointer to usize alignment
            }

            let size = *(read_pointer as *const usize) & !only_msb_of_usize;

            read_pointer = read_pointer.add(core::mem::size_of::<usize>());
            let res = ReturnedValue{
                memory: core::slice::from_raw_parts(read_pointer, size),
                buffer: self
            };
            self.read_pointer.store(read_pointer.add(size), Ordering::Release);
            Ok(res)
        }
    }
}


fn main() {
    println!("Hello, world!");
    let buff = CircullarBuffer::new();
    {

        let mut a = buff.reserve(3).expect("No i za duze");
        a[0] = 4;
        a[1] = 15;
        a[2] = 100;
        let addr = &buff.data as *const _ as usize; 
        for i in 0..BUFFER_SIZE{
            println!("({}){:#018x}: {}", i, addr+i, buff.data[i]);
        }
        println!("SECOND ROUND");
        let mut b = buff.reserve(2).expect("No i za duze");
        b[0] = 69;
        b[1] = 88;

    }
    let addr = &buff.data as *const _ as usize; 
    for i in 0..BUFFER_SIZE{
        println!("({}){:#018x}: {}", i, addr+i, buff.data[i]);
    }

    loop{
        println!("GETTING AN ELEMENT FROM BUFFER");
        let res = buff.get_value().expect("Nie ma tu nic");
        for elem in (*res).iter(){
            println!("{}", elem);
        }
    }

}

