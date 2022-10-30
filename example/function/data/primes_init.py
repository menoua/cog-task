def next_prime(i):
    def is_prime(x):
        for j in range(2, x):
            if x % j == 0:
                return False

        return True

    i += 1
    while not is_prime(i):
        i += 1

    return i
