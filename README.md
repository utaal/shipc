# Shipc

> Unpack and run oci container images.

## Usage

Requires `umoci` and `runc` to be available in `$PATH`.

To run a oci image (found in `./thisimage` either as a directory structure or a **.tar.gz** bundle),

```
shipc run thisimage
```

**Rootless** mode is supported (if you have a recent-enough runc binary):

```
shipc run thisimage --rootless
```

Bind-mounting **volumes** can be helpful, the format is the same as `docker run -v`; here's how you'd bind mount local directory `/mnt/sdb2` as the containers `/mnt/tmpstore`:

```
shipc run thisimage -v /mnt/sdb2:/mnt/tmpstore
```
