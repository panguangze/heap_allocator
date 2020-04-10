use alloc::alloc::Layout;
use core:mem::{align_of, size_of};
use core::ptr::NonNull;

use super::align_up;

// 链表节点
pub struct ListNode {
    size: usize,
    next: Option<&'static mut Hole>,
}

// 链表
pub struct MergedListAllocator {
    head: ListNode;
}

//链表节点的信息
pub struct ListNodeInfo {
    addr: usize,
    size: usize,
} 

// 将一个Node分配切割后剩余的内存信息
pub struct Allocation {
    info: ListNodeInfo,
    front_padding: Option<ListNodeInfo>,
    back_padding: Option<ListNodeInfo>,
}

impl ListNode {
    fn info(&mut self) -> ListNodeInfo {
        ListNodeInfo {
            addr: self as *const _ as usize,
            size: self.size,
        }
    }
}

impl MergedListAllocator {
    // 这个函数创建一个空的堆
    pub const fn empty() -> Self {
        Self{
            ListNode {
                size: 0,
                next: None,
            }
        }
    }
    // 这个函数根据给定的一组值创建一个堆
    pub unsafe fn new(start_addr: usize, heap_size: usize) -> Self {
        // 判断size，这里没有太明白
        assert!(size_of::<ListNode>() == Self::min_size());
        // 对其
        let aligned_node_addr = align_up(start_addr, align_of::<ListNode>());
        let ptr = aligned_node_addr as *mut ListNode;

        ptr.write(ListNode {
            size: heap_size,
            next: None,
        });

        MergedListAllocator {
            head: ListNode {
                size: 0,
                next: Some(&mut *ptr)
            }
        }
    }

    // 查找第一个足够大的ListNode来分配空间，返回该ListNode的指针。
    pub fn allocate_first_fit(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        assert!(layout.size() >= Self::min_size);

        allocate_first_fit(&mut self.head, layout).map(|allocation| {
            if let Some(padding) = allocation.front_padding {
                deallocate(&mut self.head, padding.addr, padding.size);
            }
            if let Some(padding) = allocation.back_padding {
                deallocate(&mut self.head, padding.addr, padding.size);
            }
            NonNull::new(allocation.start_addr as *mut u8).unwrap()
        })
    }

    // 内存回收，接收两个参数一个就是分配内存时返回的Node指针，另一个是node的layout信息
    pub fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        deallocate(&mut self.head, ptr.as_ptr(), layout.size())
    }

    // 最小的内存分配与回收单元
    pub fn min_size() -> usize {
        size_of::<Hole>() *2
    }
}

// 把一个listNode进行切割，返回一个Allocation对象
fn split_ListNode(listNode: ListNodeInfo, required_layout: Layout) -> Option<Allocation> {
    let required_size = required_layout.size();
    let required_align = required_layout.align();

    let (aligned_addr, front_padding) = if ListNode.addr == align_up(listNode.addr, required_align) {
        (listNode.addr, None)
    } else {
        let aligned_addr = align_up(listNode.addr + listNode::min_size(), required_align);
        (
            aligned_addr,
            Some(ListNodeInfo {
                addr: listNode.addr,
                size: aligned_addr - listNode.addr,
            })
        )
    };

    let aligned_list_node = {
        if aligned_addr + required_size > listNode.addr + listNode.size {
            return None;
        }
        ListNodeInfo {
            addr: aligned_addr,
            size: listNode.size - (aligned_addr - listNode.addr),
        }
    };

    let back_padding = if aligned_list_node.size == required_size {
        None
    } else if aligned_list_node.size < MergedListAllocator::min_size() {
        return None;
    } else {
        Some(ListNodeInfo {
            addr: aligned_list_node.addr + required_size,
            size: aligned_list_node.size - required_size,
        })
    };

    Some(Allocation {
        info: ListNodeInfo {
            addr: aligned_list_node.addr,
            size: required_size,
        }
        front_padding: front_padding,
        back_padding: back_padding,
    })
}a

//从previous.next开始向下进行检索，找到第一个足够大的ListNode然后进行切分
fn allocate_first_fit(mut previous: &mut ListNode, layout: Layout) -> Result<Allocation, ()> {
    loop {
        let allocation: Option<Allocation> = previous
            .next
            .as_mut()
            .and_then(|current| split_ListNode(current.info(), layout.clone()));
        match allocation {
            Some(allocation) => {
                previous.next = previous.next.as_mut().unwrap().next.take();
                return Ok(allocation);
            }
            None if previous.next.is_some() => {
                previous = move_helper(previous).next.as_mut().unwrap();
            }
            None => {
                return Err(());
            }
        }
    }

} 

// 回收内存
fn deallocate(mut listNode: &mut ListNode, addr: usize, mut size: usize) {
    loop {
        assert！(size >= MergedListAllocator::min_size());

        let listNode_addr = if listNode.size == 0 {
            0
        } else {
            listNode as *mut _ as usize
        };

        assert！(listNode_addr + listNode.size <= addr, "invalid deallocation(probably a double free)");

        let next_list_node_info = listNode.next.as_ref().map(|next| next.info());

        match next_list_node_info {
            Some(next) if listNode_addr + listNode.size == addr && addr + size == next.addr => {
                listNode.size += size + next.size;
                listNode.next = next.next.as_mut().unwrap().next.take();
            }
            _ if listNode_addr + listNode.size == addr => {
                listNode.size += size;
            }
            Some(next) if addr + size == next.addr => {
                listNode.next = listNode.next.as_mut().unwrap().next.take();
                size += next.size;
                continue;
            }
            Some(next) if next.addr <= addr => {
                listNode = move_helper(listNode).next.as_mut().unwrap();
                continue;
            }
            _ => {
                let new_list_node = ListNode {
                    size: size;
                    next: listNode.next.take();
                };

                debug_assert_eq!(addr % align_of::<ListNode>(), 0);
                let ptr = addr as *mut ListNode;
                unsafe {ptr.write(next_list_node_info)};
                listNode.next = Some(unsafe {&mut *ptr});
            }
        }
        break;
    }
}

fn move_helper<T>(x: T) -> T {
    x
}