import random
import os

def generate_and_save_random_numbers(min_val, max_val, count, file_path):
    """
    Generates a specified count of random integers between min_val and max_val (inclusive)
    and saves each number on a new line in the specified file_path.

    :param min_val: The inclusive minimum value for the random numbers.
    :param max_val: The inclusive maximum value for the random numbers.
    :param count: The number of random numbers to generate.
    :param file_path: The absolute or relative path to the file for storage.
    """
    # 1. Input Validation
    if min_val > max_val:
        print(f"Error: Minimum value ({min_val}) cannot be greater than the maximum value ({max_val}).")
        return
    if count <= 0:
        print(f"Error: Count must be a positive integer, got {count}.")
        return

    # 2. Generation
    random_numbers = [random.randint(min_val, max_val) for _ in range(count)]

    # 3. Storage
    try:
        # 'w' mode opens the file for writing and will create it if it doesn't exist,
        # or overwrite it if it does.
        with open(file_path, 'w') as f:
            for number in random_numbers:
                f.write(str(number) + '\n')

        # 4. Confirmation
        print("-" * 40)
        print(f"✅ Successfully generated {count} random numbers.")
        print(f"✅ Range: [{min_val}, {max_val}] (inclusive)")
        print(f"✅ Data saved to: {os.path.abspath(file_path)}")
        print("-" * 40)

    except IOError as e:
        print(f"❌ Error: Could not write to file path '{file_path}'.")
        print(f"Details: {e}")
    except Exception as e:
        print(f"An unexpected error occurred: {e}")

# --- Configuration ---
# 1. Define the range (inclusive)
MINIMUM_VALUE = -10000000
MAXIMUM_VALUE = 10000000

# 2. Define the total count of numbers to generate
NUMBER_COUNT = 500

# 3. Define the file path for storage
# You can use an absolute path (e.g., 'C:/Users/YourName/Desktop/random_data.txt')
# or a relative path (e.g., 'random_numbers.csv' which saves it in the script's directory).
OUTPUT_FILE_PATH = "data/files/data.txt" 
# ---------------------

# Execute the function
generate_and_save_random_numbers(
    MINIMUM_VALUE, 
    MAXIMUM_VALUE, 
    NUMBER_COUNT, 
    OUTPUT_FILE_PATH
)