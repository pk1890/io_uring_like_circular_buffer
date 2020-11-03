use core::mem::size_of;
use core::sync::atomic::{AtomicPtr, Ordering};


use rand::Rng;
// use alloc::boxed::Box;


const BUFFER_SIZE: usize = 4096;
const ONLY_MSB_OF_USIZE: usize = 1 << (core::mem::size_of::<usize>() * 8 - 1);

#[derive(Debug)]
pub enum BufferAddValueError {
    SizeTooBig,
}

#[derive(Debug)]
pub enum BufferGetValueError {
    NoValueInBuffer,
}

#[repr(C)]
pub struct ReservedMemory<'a> {
    memory: &'a mut [u8],
    control: &'a mut usize,
    buffer: &'a CircullarBuffer,
}

impl<'a> core::ops::Deref for ReservedMemory<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &(*self.memory)
    }
}

impl<'a> core::ops::DerefMut for ReservedMemory<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut (*self.memory)
    }
}

impl<'a> Drop for ReservedMemory<'a> {
    fn drop(&mut self) {
        *(self.control) |= ONLY_MSB_OF_USIZE;
        self.buffer.declare();
    }
}

pub unsafe fn align_ptr_to_usize(pointer: *mut u8) -> *mut u8 {
    let alignment = core::mem::align_of::<usize>();
    let modulo = (pointer as usize) % alignment;
    if modulo != 0 {
        return pointer.add(alignment - ((pointer as usize) % alignment));
    }
    pointer
}

#[repr(C)]
pub struct ReturnedValue<'a> {
    pub memory: &'a [u8],
    control: &'a mut usize,
    buffer: &'a CircullarBuffer,
}

impl<'a> ReturnedValue<'a>{
    pub fn get_ref(&self) ->  &'a [u8]{
        self.memory
    }
    pub fn get_size(&self) -> usize{
        *self.control & !ONLY_MSB_OF_USIZE
    }
}

impl<'a> core::ops::Deref for ReturnedValue<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &(*self.memory)
    }
}

impl<'a> Drop for ReturnedValue<'a> {
    fn drop(&mut self) {
        *(self.control) |= ONLY_MSB_OF_USIZE;
        self.buffer.release()
    }
}

#[repr(C)]
pub struct CircullarBuffer {
    data: Box<[u8; 2*BUFFER_SIZE]>,
    // additional_data: [u8; BUFFER_SIZE],
    read_pointer: AtomicPtr<u8>,
    write_pointer: AtomicPtr<u8>,
    release_pointer: AtomicPtr<u8>,
    reservation_pointer: AtomicPtr<u8>,
}

impl CircullarBuffer {
    pub fn new() -> Self {
        let mut buff = CircullarBuffer {
            data: Box::new([0; 2*BUFFER_SIZE]),
            // additional_data: [0; BUFFER_SIZE],
            reservation_pointer: AtomicPtr::new(&mut 0),
            write_pointer: AtomicPtr::new(&mut 0),
            read_pointer: AtomicPtr::new(&mut 0),
            release_pointer: AtomicPtr::new(&mut 0),
        };

        let address = buff.data.as_mut_ptr() as *mut _ as *mut u8;

        unsafe {
            let start_address = align_ptr_to_usize(address);
            buff.reservation_pointer
                .store(start_address, Ordering::Relaxed);
            buff.write_pointer.store(start_address, Ordering::Relaxed);
            buff.read_pointer.store(start_address, Ordering::Relaxed);
            buff.release_pointer.store(start_address, Ordering::Relaxed);
        }
        // //println!("Buffer_address: {:#018x}", address as u64);
        buff.print_status();
        buff
    }


    pub fn print_status(&self) {
        println!(
            "\tres: {:#018x},\twrite: {:#018x},\tread: {:#018x},\trel: {:#018x}",
            self.reservation_pointer.load(Ordering::Acquire) as usize,
            self.write_pointer.load(Ordering::Acquire) as usize,
            self.read_pointer.load(Ordering::Acquire) as usize,
            self.release_pointer.load(Ordering::Acquire) as usize
        );
        println!("addr: {:#018x} / {:#018x} half: {:#018x}, end: {:#018x}", self as *const _ as usize, self.data.as_ptr() as usize, self.data.as_ptr() as usize + BUFFER_SIZE, self.data.as_ptr() as usize + 2*BUFFER_SIZE);
    }


    pub fn isEmpty(&self) -> bool{
        self.write_pointer.load(Ordering::Acquire) as u64 == self.read_pointer.load(Ordering::Acquire) as u64 
    }

    pub fn reserve(&self, size: usize) -> Result<ReservedMemory, BufferAddValueError> {
        //print!("START RESERVATION");
        self.print_status();


        if size & ONLY_MSB_OF_USIZE != 0 {
            return Err(BufferAddValueError::SizeTooBig);
        }
        unsafe {
            let mut pointer = align_ptr_to_usize(self.reservation_pointer.load(Ordering::Acquire));

            let end_of_buffer = self.data.as_ptr() as usize + BUFFER_SIZE;
            let release_pointer = self.release_pointer.load(Ordering::Acquire);
            if (release_pointer as usize) + BUFFER_SIZE
                - (pointer as usize)
                - core::mem::size_of::<usize>()
                < size
            {
                return Err(BufferAddValueError::SizeTooBig);
            }

            core::ptr::copy_nonoverlapping(&size as *const usize, pointer as *mut usize, 1);
            let control_usize = &mut *(pointer as *mut usize);

            pointer = pointer.add(size_of::<usize>()); //set pointer after inserted usize
            let res = ReservedMemory {
                memory: core::slice::from_raw_parts_mut::<u8>(pointer, size),
                control: control_usize,
                buffer: self,
            };

            pointer = align_ptr_to_usize(pointer.add(size));

            if pointer as usize >= end_of_buffer {
                self.reservation_pointer
                    .store(pointer.sub(BUFFER_SIZE), Ordering::Release);
            } else {
                self.reservation_pointer.store(pointer, Ordering::Release);
            }
            //print!("END RESERVATION      ");
            self.print_status();
            Ok(res)
        }
    }

    fn declare(&self) {
        //print!("START DECLARATION");
        self.print_status();
        if self.reservation_pointer.load(Ordering::Acquire) as u64
            == self.write_pointer.load(Ordering::Acquire) as u64
        {
            return;
        }
        let mut pointer = self.write_pointer.load(Ordering::Acquire);
        let mut changed = false;
        let end_of_buffer = self.data.as_ptr() as usize + BUFFER_SIZE;

        unsafe {
            loop {
                pointer = align_ptr_to_usize(pointer);
                let size_ref = &mut *(pointer as *mut usize);

                if *size_ref & ONLY_MSB_OF_USIZE == 0 {
                    if changed {
                        
                        if pointer as usize >= end_of_buffer {
                            self.write_pointer
                                .store(pointer.sub(BUFFER_SIZE), Ordering::Release);
                        } else {
                            self.write_pointer.store(pointer, Ordering::Release);
                        }
                    }

                    //print!("END DECLARATION");
                    self.print_status();

                    return;
                }

                let size = *size_ref & !ONLY_MSB_OF_USIZE;
                *size_ref &= !ONLY_MSB_OF_USIZE; // zero the control bit

                pointer = pointer.add(core::mem::size_of::<usize>() + size);
                changed = true;
            }
        }
    }

    pub fn get_value(&self) -> Result<ReturnedValue, BufferGetValueError> {
        unsafe {
            //print!("START GETVAL");
            self.print_status();

            let mut read_pointer = self.read_pointer.load(Ordering::Acquire);
            let write_pointer = self.write_pointer.load(Ordering::Acquire);

            let end_of_buffer = self.data.as_ptr() as usize + BUFFER_SIZE;

            if read_pointer as usize == write_pointer as usize {
                return Err(BufferGetValueError::NoValueInBuffer);
            }

            let size = *(read_pointer as *const usize) & !ONLY_MSB_OF_USIZE;
            let control_usize = &mut *(read_pointer as *mut usize);

            read_pointer = read_pointer.add(core::mem::size_of::<usize>());
            let res = ReturnedValue {
                memory: core::slice::from_raw_parts(read_pointer, size),
                control: control_usize,
                buffer: self,
            };

            read_pointer = align_ptr_to_usize(read_pointer.add(size));

            if read_pointer as usize >= end_of_buffer {
                self.read_pointer
                    .store(read_pointer.sub(BUFFER_SIZE), Ordering::Release);
            } else {
                self.read_pointer.store(read_pointer, Ordering::Release);
            }

            //print!("END GETVAL");
            self.print_status();
            Ok(res)
        }
    }

    pub fn release(&self) {
        //print!("START RELEASE");
        self.print_status();

        if self.release_pointer.load(Ordering::Acquire) as u64
            == self.read_pointer.load(Ordering::Acquire) as u64
        {
            return;
        }
        let mut pointer = self.release_pointer.load(Ordering::Acquire);
        let end_of_buffer = self.data.as_ptr() as usize + BUFFER_SIZE;
        let mut changed = false;
        unsafe {
            loop {
                pointer = align_ptr_to_usize(pointer);

                let size_ref = &mut *(pointer as *mut usize);

                if *size_ref & ONLY_MSB_OF_USIZE == 0 {
                    if changed {
                        if pointer as usize >= end_of_buffer {
                            self.release_pointer
                                .store(pointer.sub(BUFFER_SIZE), Ordering::Release);
                        } else {
                            self.release_pointer.store(pointer, Ordering::Release);
                        }
                    }

                    //print!("END RELEASE");
                    self.print_status();

                    return;
                }

                let size = *size_ref & !ONLY_MSB_OF_USIZE;
                *size_ref &= !ONLY_MSB_OF_USIZE; // zero the control bit

                pointer = pointer.add(core::mem::size_of::<usize>() + size);
                changed = true;
            }
        }
    }
}

fn main() {
    println!("Hello, world!");
    let buff = CircullarBuffer::new();
    let mut rng = rand::thread_rng();
    let mut j: u8 = 0;
    for i in 0u64..20000 {
        j += 1;
        if j > 250 {
            j = 0;
        }
        {
            let mut a = buff.reserve(rng.gen_range(4, 400)).expect("No i za duze");
            a[0] = j;
            a[1] = j + 1;
            a[2] = j + 2;
        }
        {
            let mut a = buff.reserve(rng.gen_range(4, 400)).expect("No i za duze");
            a[0] = j;
            a[1] = j + 1;
            a[2] = j + 2;
        }
        {
            let res = buff.get_value().expect("Nie ma tu nic");
            for elem in (*res).iter() {
                println!("{}", elem);
            }
        }
        {
            let res = buff.get_value().expect("Nie ma tu nic");
            for elem in (*res).iter() {
                println!("{}", elem);
            }
        }
        println!("{}", i);
    }

    println!("OK!");
}
