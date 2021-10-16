//! Places data on a matrix
#![deny(unsafe_code)]
#![warn(missing_docs)]

use crate::bitstring::BitString;
use crate::datamasking;
use crate::default;
use crate::encode;
use crate::encode::Mode;
use crate::helpers;
use crate::polynomials;
use crate::score;
use crate::vecl;
use crate::vecl::ECL;
use crate::version::Version;

/// Places the data on the matrix
const fn place_on_matrix_data<const N: usize>(
    mut mat: [[bool; N]; N],
    structure_as_binarystring: BitString<5430>,
    version: Version,
) -> [[bool; N]; N] {
    let mat_full: [[bool; N]; N] = default::non_available_matrix_from_version(version);

    let mut direction: i8 = -1;

    let dimension = (version as usize) * 4 + 17;
    let [mut x, mut y]: [i32; 2] = [dimension as i32 - 1, dimension as i32 - 1];

    let structure_bytes_tmp = structure_as_binarystring.get_data();

    let mut i = 0;
    loop {
        if y < 0 {
            y = 0;
            direction = 1;
            x -= 2;
        }
        if y >= dimension as i32 {
            y = dimension as i32 - 1;
            direction = -1;
            x -= 2;
        }
        if x == 6 {
            x -= 1;
        }

        if x < 0 {
            break;
        }
        if !mat_full[y as usize][x as usize] {
            let c = structure_bytes_tmp[i / 8] & (1 << (7 - i % 8));
            i += 1;
            mat[y as usize][x as usize] = c != 0;
        }
        if !mat_full[y as usize][x as usize - 1] {
            let c = structure_bytes_tmp[i / 8] & (1 << (7 - i % 8));
            i += 1;
            mat[y as usize][x as usize - 1] = c != 0;
        }

        y += direction as i32;
    }

    mat
}

/// Placement the format information for all QRCodes
const fn place_on_matrix_formatinfo<const N: usize>(
    mut mat: [[bool; N]; N],
    formatinfo: u16,
) -> [[bool; N]; N] {
    let length = mat.len();

    let mut i = 6;
    while i > 0 {
        i -= 1;

        let shift = 1 << (i + 9);
        let value = (formatinfo & shift) != 0;
        mat[8][i] = value;
        mat[length - 6 + i][8] = value;
    }

    let mut i = 0;
    while i <= 5 {
        let shift = 1 << i;
        let value = (formatinfo & shift) != 0;
        mat[i][8] = value;
        mat[8][length - i - 1] = value;

        i += 1;
    }

    {
        let shift = 1 << 6;
        let value = (formatinfo & shift) != 0;
        // Six on left
        mat[8][7] = value;
        // Six on bottom
        mat[length - 7][7] = value;
    }
    {
        let shift = 1 << 7;
        let value = (formatinfo & shift) != 0;
        // Seven on left
        mat[8][8] = value;
        // Seven on right
        mat[8][length - 8] = value;
    }
    {
        let shift = 1 << 8;
        let value = (formatinfo & shift) != 0;
        // Height on left
        mat[7][8] = value;
        // Height on right
        mat[8][length - 7] = value;
    }

    return mat;
}

/// Places version information for QRCodes larger and equal to version 7
const fn place_on_matrix_versioninfo<const N: usize>(
    mut mat: [[bool; N]; N],
    version: Version,
) -> [[bool; N]; N] {
    if (version as usize) < 7 {
        return mat;
    }

    let length = mat.len();

    let version_info = version.information();

    let mut i = 0;
    while i <= 2 {
        let mut j = 0;
        while j <= 5 {
            let shift: u32 = 1 << (j * 3 + i);
            mat[j][length - 11 + i] = (version_info & shift) != 0;
            mat[length - 11 + i][j] = (version_info & shift) != 0;

            j += 1;
        }

        i += 1;
    }

    return mat;
}

/// Main function to place everything in the QRCode, returns a valid matrix
pub const fn place_on_matrix<const N: usize>(
    structure_as_binarystring: BitString<5430>,
    version: Version,
    quality: vecl::ECL,
) -> [[bool; N]; N] {
    let mut best_score = u32::MAX;
    let mut best_mask = u8::MAX;

    let mat = [[false; N]; N];
    let version = version;

    let mat = default::create_matrix_pattern(mat);
    let mat = default::create_matrix_timing(mat);
    let mat = default::create_matrix_black_module(mat, version);
    let mat = default::create_matrix_alignments(mat, version);
    let mat = place_on_matrix_data(mat, structure_as_binarystring, version);
    let mut mat = place_on_matrix_versioninfo(mat, version);

    // Taken out from mask, that is used 8*2 + 1 times in this function
    let mat_full = default::non_available_matrix_from_version(version);

    let mut mask_nb = 0;

    while mask_nb < 8 {
        mat = datamasking::mask(mat, mask_nb, &mat_full);
        let matrix_score = score::matrix_score(&mat);
        if matrix_score < best_score {
            best_score = matrix_score;
            best_mask = mask_nb;
        }
        mat = datamasking::mask(mat, mask_nb, &mat_full);

        mask_nb += 1;
    }

    let encoded_format_info = vecl::ecm_to_format_information(quality, best_mask as usize);
    mat = place_on_matrix_formatinfo(mat, encoded_format_info);
    mat = datamasking::mask(mat, best_mask, &mat_full);
    mat
}

/// Generate the whole matrix
pub const fn create_matrix<const N: usize>(
    input: &[u8],
    ecl: ECL,
    mode: Mode,
    version: Version,
) -> [[bool; N]; N] {
    let data_codewords = encode::encode(input, ecl, mode, version);

    let error_codewords = version.get_polynomial(ecl);

    let structure =
        polynomials::structure(&data_codewords.get_data(), &error_codewords, ecl, version);

    let structure_binstring = helpers::binary_to_binarystring_version(structure, version, ecl);

    place_on_matrix(structure_binstring, version, ecl)
}
