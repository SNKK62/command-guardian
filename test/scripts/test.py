import tqdm
import time

start = time.time()
for i in tqdm.tqdm(range(100000000)):
    if i % 10000000 == 0:
        print(i)
    pass

print(time.time() - start)
