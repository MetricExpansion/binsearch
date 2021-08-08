#include <iostream>

extern "C" {
    void search(const unsigned char** found_range, std::size_t* found_len, const unsigned char* data, std::size_t length, float min, bool use_min, float max, bool use_max, std::size_t min_length);
}

inline bool is_valid_value(float value, float min, float max, bool use_min, bool use_max) {
    if ((!use_min || value >= min) && (!use_max || value <= max)) {
        return true;
    } else {
        return false;
    }
}

void search(const unsigned char** found_range, std::size_t* found_len, const unsigned char* data, std::size_t length, float min, bool use_min, float max, bool use_max, std::size_t min_length) {
    const unsigned char* pos = data;
    const unsigned char* end = data + length;

    const unsigned char* valid_run_pos = nullptr;

    while (pos + 4 <= end) {
        float value = *((float*) pos);
        if (is_valid_value(value, min, max, use_min, use_max)) {
            if (!valid_run_pos) {
                valid_run_pos = pos;
            }
        } else {
            if (valid_run_pos) {
                if ((unsigned long)(pos - valid_run_pos) >= (sizeof(float)*min_length)) {
                    *found_range = valid_run_pos;
                    *found_len = (pos - valid_run_pos);
                    return;
                } else {
                    valid_run_pos = nullptr;
                }
            }
        }
        pos = pos + sizeof(float);
    }
    if (valid_run_pos && pos < end) {
        if ((unsigned long)(pos - valid_run_pos) >= (sizeof(float)*min_length)) {
            *found_range = valid_run_pos;
            *found_len = (pos - valid_run_pos);
            return;
        } else {
            valid_run_pos = nullptr;
        }
    }
}

