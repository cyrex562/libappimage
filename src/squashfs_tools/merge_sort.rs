/// Trait for types that can be sorted by name
pub trait SortableByName {
    /// Get the name field for comparison
    fn name(&self) -> &str;
    /// Get the next item in the linked list
    fn next(&mut self) -> &mut Option<Box<Self>>;
}

/// Sort a linked list of items by name using bottom-up merge sort
/// 
/// This implementation is optimized for linked lists rather than arrays.
/// It uses a bottom-up approach that:
/// 1. Starts with single-element sublists (which are trivially sorted)
/// 2. Merges adjacent sublists of increasing size
/// 3. Continues until the entire list is sorted
/// 
/// # Arguments
/// * `head` - Reference to the head of the linked list
/// * `count` - Number of elements in the list
pub fn sort_by_name<T: SortableByName>(head: &mut Option<Box<T>>, count: usize) {
    if head.is_none() || count < 2 {
        return;
    }

    let mut stride = 1;

    // Each iteration merges adjacent sublists of size stride into sublists of size 2*stride
    while stride < count {
        let mut l2 = head.take(); // Current head of list
        let mut cur = None; // Output list

        // Iterate through the list, merging adjacent sublists
        while let Some(mut l2_node) = l2 {
            let mut l1 = l2_node;
            let mut len1 = 0;
            let mut len2 = stride;

            // Get first sublist of size stride
            while len1 < stride {
                if let Some(next) = l1.next().take() {
                    l1 = next;
                    len1 += 1;
                } else {
                    break;
                }
            }

            // Get second sublist of size stride
            l2 = l1.next().take();

            // Merge the two sublists
            while len1 > 0 && l2.is_some() && len2 > 0 {
                let l1_name = l1.name();
                let l2_name = l2.as_ref().unwrap().name();

                let next = if l1_name <= l2_name {
                    len1 -= 1;
                    let next = l1.next().take();
                    l1
                } else {
                    len2 -= 1;
                    let next = l2.as_mut().unwrap().next().take();
                    l2.take().unwrap()
                };

                // Add to output list
                if let Some(cur_node) = &mut cur {
                    cur_node.next().replace(Box::new(next));
                    cur = cur_node.next().as_mut();
                } else {
                    cur = Some(Box::new(next));
                    head = cur.as_mut();
                }
            }

            // Copy remaining elements from first sublist
            while len1 > 0 {
                if let Some(next) = l1.next().take() {
                    l1 = next;
                    len1 -= 1;

                    // Add to output list
                    if let Some(cur_node) = &mut cur {
                        cur_node.next().replace(Box::new(l1));
                        cur = cur_node.next().as_mut();
                    } else {
                        cur = Some(Box::new(l1));
                        head = cur.as_mut();
                    }
                } else {
                    break;
                }
            }

            // Copy remaining elements from second sublist
            while l2.is_some() && len2 > 0 {
                let next = l2.as_mut().unwrap().next().take();
                let node = l2.take().unwrap();
                len2 -= 1;

                // Add to output list
                if let Some(cur_node) = &mut cur {
                    cur_node.next().replace(node);
                    cur = cur_node.next().as_mut();
                } else {
                    cur = Some(node);
                    head = cur.as_mut();
                }

                l2 = next;
            }
        }

        // Terminate the list
        if let Some(cur_node) = &mut cur {
            cur_node.next().take();
        }

        stride <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test structure implementing SortableByName
    #[derive(Debug)]
    struct TestNode {
        name: String,
        next: Option<Box<TestNode>>,
    }

    impl SortableByName for TestNode {
        fn name(&self) -> &str {
            &self.name
        }

        fn next(&mut self) -> &mut Option<Box<Self>> {
            &mut self.next
        }
    }

    // Helper function to create a test list
    fn create_test_list(names: &[&str]) -> Option<Box<TestNode>> {
        let mut head = None;
        for &name in names.iter().rev() {
            head = Some(Box::new(TestNode {
                name: name.to_string(),
                next: head,
            }));
        }
        head
    }

    // Helper function to collect list items
    fn collect_list(head: &Option<Box<TestNode>>) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = head;
        while let Some(node) = current {
            result.push(node.name.clone());
            current = &node.next;
        }
        result
    }

    #[test]
    fn test_sort_empty() {
        let mut head = None;
        sort_by_name(&mut head, 0);
        assert!(head.is_none());
    }

    #[test]
    fn test_sort_single() {
        let mut head = create_test_list(&["test"]);
        sort_by_name(&mut head, 1);
        assert_eq!(collect_list(&head), vec!["test"]);
    }

    #[test]
    fn test_sort_multiple() {
        let mut head = create_test_list(&["c", "a", "b", "d"]);
        sort_by_name(&mut head, 4);
        assert_eq!(collect_list(&head), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_sort_duplicates() {
        let mut head = create_test_list(&["b", "a", "b", "a"]);
        sort_by_name(&mut head, 4);
        assert_eq!(collect_list(&head), vec!["a", "a", "b", "b"]);
    }

    #[test]
    fn test_sort_large() {
        let names: Vec<&str> = (0..100)
            .map(|i| format!("item{}", i))
            .collect::<Vec<_>>()
            .iter()
            .map(|s| s.as_str())
            .collect();
        
        let mut head = create_test_list(&names);
        sort_by_name(&mut head, 100);
        
        let result = collect_list(&head);
        assert_eq!(result.len(), 100);
        assert!(result.windows(2).all(|w| w[0] <= w[1]));
    }
} 