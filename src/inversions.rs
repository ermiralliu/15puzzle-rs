/// Merge-sort-based inversion count. Returns number of inversions in `arr`
/// (ignoring zeros). Sorts `arr` in place as a side-effect.
pub fn count(arr: &mut [u8]) -> u32 {
    let len = arr.len();
    if len <= 1 {
        return 0;
    }
    let mid = len / 2;
    let left_inv = count(&mut arr[..mid]);
    let right_inv = count(&mut arr[mid..]);
    let merge_inv = merge(arr, mid);
    left_inv + right_inv + merge_inv
}

fn merge(arr: &mut [u8], mid: usize) -> u32 {
    let left = arr[..mid].to_vec();
    let right = arr[mid..].to_vec();
    let mut i = 0;
    let mut j = 0;
    let mut k = 0;
    let mut inv = 0u32;
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            arr[k] = left[i];
            i += 1;
        } else {
            arr[k] = right[j];
            inv += (left.len() - i) as u32;
            j += 1;
        }
        k += 1;
    }
    while i < left.len() {
        arr[k] = left[i];
        i += 1;
        k += 1;
    }
    while j < right.len() {
        arr[k] = right[j];
        j += 1;
        k += 1;
    }
    inv
}
