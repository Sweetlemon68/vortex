#ifndef SPIRAL_ZIMD_H
#define SPIRAL_ZIMD_H

#include "stdint.h"
#include "stdbool.h"
#include "float.h"

#if defined(__cplusplus)
extern "C" {
#endif

// max
uint8_t zimd_max_u8(uint8_t const *const ptr, uintptr_t const len);
uint16_t zimd_max_u16(uint16_t const *const ptr, uintptr_t const len);
uint32_t zimd_max_u32(uint32_t const *const ptr, uintptr_t const len);
uint64_t zimd_max_u64(uint64_t const *const ptr, uintptr_t const len);
int8_t zimd_max_i8(int8_t const *const ptr, uintptr_t const len);
int16_t zimd_max_i16(int16_t const *const ptr, uintptr_t const len);
int32_t zimd_max_i32(int32_t const *const ptr, uintptr_t const len);
int64_t zimd_max_i64(int64_t const *const ptr, uintptr_t const len);
float zimd_max_f32(float const *const ptr, uintptr_t const len);
double zimd_max_f64(double const *const ptr, uintptr_t const len);

// min
uint8_t zimd_min_u8(uint8_t const *const ptr, uintptr_t const len);
uint16_t zimd_min_u16(uint16_t const *const ptr, uintptr_t const len);
uint32_t zimd_min_u32(uint32_t const *const ptr, uintptr_t const len);
uint64_t zimd_min_u64(uint64_t const *const ptr, uintptr_t const len);
int8_t zimd_min_i8(int8_t const *const ptr, uintptr_t const len);
int16_t zimd_min_i16(int16_t const *const ptr, uintptr_t const len);
int32_t zimd_min_i32(int32_t const *const ptr, uintptr_t const len);
int64_t zimd_min_i64(int64_t const *const ptr, uintptr_t const len);
float zimd_min_f32(float const *const ptr, uintptr_t const len);
double zimd_min_f64(double const *const ptr, uintptr_t const len);

// isSorted
bool zimd_isSorted_u8(uint8_t const *const ptr, uintptr_t const len);
bool zimd_isSorted_u16(uint16_t const *const ptr, uintptr_t const len);
bool zimd_isSorted_u32(uint32_t const *const ptr, uintptr_t const len);
bool zimd_isSorted_u64(uint64_t const *const ptr, uintptr_t const len);
bool zimd_isSorted_i8(int8_t const *const ptr, uintptr_t const len);
bool zimd_isSorted_i16(int16_t const *const ptr, uintptr_t const len);
bool zimd_isSorted_i32(int32_t const *const ptr, uintptr_t const len);
bool zimd_isSorted_i64(int64_t const *const ptr, uintptr_t const len);
bool zimd_isSorted_f32(float const *const ptr, uintptr_t const len);
bool zimd_isSorted_f64(double const *const ptr, uintptr_t const len);

// isConstant
bool zimd_isConstant_u8(uint8_t const *const ptr, uintptr_t const len);
bool zimd_isConstant_u16(uint16_t const *const ptr, uintptr_t const len);
bool zimd_isConstant_u32(uint32_t const *const ptr, uintptr_t const len);
bool zimd_isConstant_u64(uint64_t const *const ptr, uintptr_t const len);
bool zimd_isConstant_i8(int8_t const *const ptr, uintptr_t const len);
bool zimd_isConstant_i16(int16_t const *const ptr, uintptr_t const len);
bool zimd_isConstant_i32(int32_t const *const ptr, uintptr_t const len);
bool zimd_isConstant_i64(int64_t const *const ptr, uintptr_t const len);
bool zimd_isConstant_f32(float const *const ptr, uintptr_t const len);
bool zimd_isConstant_f64(double const *const ptr, uintptr_t const len);

// runLengthStats
typedef struct {
    uint64_t runCount;
    uint64_t runElementCount;
} RunLengthStats_t;

RunLengthStats_t zimd_runLengthStats_u8(uint8_t const *const ptr, uintptr_t const len);
RunLengthStats_t zimd_runLengthStats_u16(uint16_t const *const ptr, uintptr_t const len);
RunLengthStats_t zimd_runLengthStats_u32(uint32_t const *const ptr, uintptr_t const len);
RunLengthStats_t zimd_runLengthStats_u64(uint64_t const *const ptr, uintptr_t const len);
RunLengthStats_t zimd_runLengthStats_i8(int8_t const *const ptr, uintptr_t const len);
RunLengthStats_t zimd_runLengthStats_i16(int16_t const *const ptr, uintptr_t const len);
RunLengthStats_t zimd_runLengthStats_i32(int32_t const *const ptr, uintptr_t const len);
RunLengthStats_t zimd_runLengthStats_i64(int64_t const *const ptr, uintptr_t const len);
RunLengthStats_t zimd_runLengthStats_f32(float const *const ptr, uintptr_t const len);
RunLengthStats_t zimd_runLengthStats_f64(double const *const ptr, uintptr_t const len);

#if defined(__cplusplus)
} // extern "C"
#endif

#endif // SPIRAL_ZIMD_H