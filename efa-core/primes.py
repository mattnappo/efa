def find(n):
    for i in range(2, n):
        prime = True
        for j in range(2, i):
            if i % j == 0:
                prime = False
        if prime:
            print(i)

def find2(n):
    i = 2
    while i < n:
        prime = True
        j = 2
        while j < i:
            if i % j == 0:
                prime = False
            j += 1
        if prime:
            print(i)

        i += 1

# find(10000)
find2(50)

