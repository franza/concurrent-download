This project downloads target URL concurrently using tokio and futures functionality.
This is just an exercise and is not intended as prod-ready or whatever-ready project.

Usage:

```shell
concurrent-download -t 4 -d ubuntu.iso https://ftp.halifax.rwth-aachen.de/ubuntu-releases/20.04.2.0/ubuntu-20.04.2.0-desktop-amd64.iso
```

where `-t` is number of concurrent threads (not system threads though, everything is decided by tokio) and `-d` is destination file. 

Files are not merged together after the download right now, may be implemented in future.

However, to memorize how downloading concurrently improves the speed here are the results (it does not improve much though):

```shell
# 4 concurrent theads
$ time ./concurrent-download -t 4 -d ubuntu.iso https://ftp.halifax.rwth-aachen.de/ubuntu-releases/20.04.2.0/ubuntu-20.04.2.0-desktop-amd64.iso
downloading chunk 0-719306752 of 2877227008
downloading chunk 719306752-1438613504 of 2877227008
downloading chunk 1438613504-2157920256 of 2877227008
downloading chunk 2157920256-2877227008 of 2877227008
chunk 0-719306752 done
chunk 2157920256-2877227008 done
chunk 719306752-1438613504 done
chunk 1438613504-2157920256 done
done

real    5m47,582s
user    1m37,861s
sys     2m31,044s

# 1 thread
$ time ./concurrent-download -t 1 -d ubuntu.iso https://ftp.halifax.rwth-aachen.de/ubuntu-releases/20.04.2.0/ubuntu-20.04.2.0-desktop-amd64.iso
downloading chunk 0-2877227008 of 2877227008
chunk 0-2877227008 done
done

real    5m58,579s
user    1m16,006s
sys     1m52,515s
```
