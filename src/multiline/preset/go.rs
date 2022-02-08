#[cfg(test)]
mod tests {
    #[test]
    #[allow(clippy::missing_const_for_fn)]
    fn merge() {
        let input = [
            "panic: my panic\n",
            "\n",
            "goroutine 4 [running]:\n",
            "panic(0x45cb40, 0x47ad70)\n",
            "	/usr/local/go/src/runtime/panic.go:542 +0x46c fp=0xc42003f7b8 sp=0xc42003f710 pc=0x422f7c\n",
            "main.main.func1(0xc420024120)\n",
            "	foo.go:6 +0x39 fp=0xc42003f7d8 sp=0xc42003f7b8 pc=0x451339\n",
            "runtime.goexit()\n",
            "	/usr/local/go/src/runtime/asm_amd64.s:2337 +0x1 fp=0xc42003f7e0 sp=0xc42003f7d8 pc=0x44b4d1\n",
            "created by main.main\n",
            "	foo.go:5 +0x58\n",
            "\n",
            "goroutine 1 [chan receive]:\n",
            "runtime.gopark(0x4739b8, 0xc420024178, 0x46fcd7, 0xc, 0xc420028e17, 0x3)\n",
            "	/usr/local/go/src/runtime/proc.go:280 +0x12c fp=0xc420053e30 sp=0xc420053e00 pc=0x42503c\n",
            "runtime.goparkunlock(0xc420024178, 0x46fcd7, 0xc, 0x1000f010040c217, 0x3)\n",
            "	/usr/local/go/src/runtime/proc.go:286 +0x5e fp=0xc420053e70 sp=0xc420053e30 pc=0x42512e\n",
            "runtime.chanrecv(0xc420024120, 0x0, 0xc420053f01, 0x4512d8)\n",
            "	/usr/local/go/src/runtime/chan.go:506 +0x304 fp=0xc420053f20 sp=0xc420053e70 pc=0x4046b4\n",
            "runtime.chanrecv1(0xc420024120, 0x0)\n",
            "	/usr/local/go/src/runtime/chan.go:388 +0x2b fp=0xc420053f50 sp=0xc420053f20 pc=0x40439b\n",
            "main.main()\n",
            "	foo.go:9 +0x6f fp=0xc420053f80 sp=0xc420053f50 pc=0x4512ef\n",
            "runtime.main()\n",
            "	/usr/local/go/src/runtime/proc.go:185 +0x20d fp=0xc420053fe0 sp=0xc420053f80 pc=0x424bad\n",
            "runtime.goexit()\n",
            "	/usr/local/go/src/runtime/asm_amd64.s:2337 +0x1 fp=0xc420053fe8 sp=0xc420053fe0 pc=0x44b4d1\n",
            "\n",
            "goroutine 2 [force gc (idle)]:\n",
            "runtime.gopark(0x4739b8, 0x4ad720, 0x47001e, 0xf, 0x14, 0x1)\n",
            "	/usr/local/go/src/runtime/proc.go:280 +0x12c fp=0xc42003e768 sp=0xc42003e738 pc=0x42503c\n",
            "runtime.goparkunlock(0x4ad720, 0x47001e, 0xf, 0xc420000114, 0x1)\n",
            "	/usr/local/go/src/runtime/proc.go:286 +0x5e fp=0xc42003e7a8 sp=0xc42003e768 pc=0x42512e\n",
            "runtime.forcegchelper()\n",
            "	/usr/local/go/src/runtime/proc.go:238 +0xcc fp=0xc42003e7e0 sp=0xc42003e7a8 pc=0x424e5c\n",
            "runtime.goexit()\n",
            "	/usr/local/go/src/runtime/asm_amd64.s:2337 +0x1 fp=0xc42003e7e8 sp=0xc42003e7e0 pc=0x44b4d1\n",
            "created by runtime.init.4\n",
            "	/usr/local/go/src/runtime/proc.go:227 +0x35\n",
            "\n",
            "goroutine 3 [GC sweep wait]:\n",
            "runtime.gopark(0x4739b8, 0x4ad7e0, 0x46fdd2, 0xd, 0x419914, 0x1)\n",
            "	/usr/local/go/src/runtime/proc.go:280 +0x12c fp=0xc42003ef60 sp=0xc42003ef30 pc=0x42503c\n",
            "runtime.goparkunlock(0x4ad7e0, 0x46fdd2, 0xd, 0x14, 0x1)\n",
            "	/usr/local/go/src/runtime/proc.go:286 +0x5e fp=0xc42003efa0 sp=0xc42003ef60 pc=0x42512e\n",
            "runtime.bgsweep(0xc42001e150)\n",
            "	/usr/local/go/src/runtime/mgcsweep.go:52 +0xa3 fp=0xc42003efd8 sp=0xc42003efa0 pc=0x419973\n",
            "runtime.goexit()\n",
            "	/usr/local/go/src/runtime/asm_amd64.s:2337 +0x1 fp=0xc42003efe0 sp=0xc42003efd8 pc=0x44b4d1\n",
            "created by runtime.gcenable\n",
            "	/usr/local/go/src/runtime/mgc.go:216 +0x58\n",
            "one more line, no multiline",
        ];

        let want = [
            "panic: my panic\n
\n
goroutine 4 [running]:\n
panic(0x45cb40, 0x47ad70)\n
	/usr/local/go/src/runtime/panic.go:542 +0x46c fp=0xc42003f7b8 sp=0xc42003f710 pc=0x422f7c\n
main.main.func1(0xc420024120)\n
	foo.go:6 +0x39 fp=0xc42003f7d8 sp=0xc42003f7b8 pc=0x451339\n
runtime.goexit()\n
	/usr/local/go/src/runtime/asm_amd64.s:2337 +0x1 fp=0xc42003f7e0 sp=0xc42003f7d8 pc=0x44b4d1\n
created by main.main\n
	foo.go:5 +0x58\n
\n
goroutine 1 [chan receive]:\n
runtime.gopark(0x4739b8, 0xc420024178, 0x46fcd7, 0xc, 0xc420028e17, 0x3)\n
	/usr/local/go/src/runtime/proc.go:280 +0x12c fp=0xc420053e30 sp=0xc420053e00 pc=0x42503c\n
runtime.goparkunlock(0xc420024178, 0x46fcd7, 0xc, 0x1000f010040c217, 0x3)\n
	/usr/local/go/src/runtime/proc.go:286 +0x5e fp=0xc420053e70 sp=0xc420053e30 pc=0x42512e\n
runtime.chanrecv(0xc420024120, 0x0, 0xc420053f01, 0x4512d8)\n
	/usr/local/go/src/runtime/chan.go:506 +0x304 fp=0xc420053f20 sp=0xc420053e70 pc=0x4046b4\n
runtime.chanrecv1(0xc420024120, 0x0)\n
	/usr/local/go/src/runtime/chan.go:388 +0x2b fp=0xc420053f50 sp=0xc420053f20 pc=0x40439b\n
main.main()\n
	foo.go:9 +0x6f fp=0xc420053f80 sp=0xc420053f50 pc=0x4512ef\n
runtime.main()\n
	/usr/local/go/src/runtime/proc.go:185 +0x20d fp=0xc420053fe0 sp=0xc420053f80 pc=0x424bad\n
runtime.goexit()\n
	/usr/local/go/src/runtime/asm_amd64.s:2337 +0x1 fp=0xc420053fe8 sp=0xc420053fe0 pc=0x44b4d1\n
\n
goroutine 2 [force gc (idle)]:\n
runtime.gopark(0x4739b8, 0x4ad720, 0x47001e, 0xf, 0x14, 0x1)\n
	/usr/local/go/src/runtime/proc.go:280 +0x12c fp=0xc42003e768 sp=0xc42003e738 pc=0x42503c\n
runtime.goparkunlock(0x4ad720, 0x47001e, 0xf, 0xc420000114, 0x1)\n
	/usr/local/go/src/runtime/proc.go:286 +0x5e fp=0xc42003e7a8 sp=0xc42003e768 pc=0x42512e\n
runtime.forcegchelper()\n
	/usr/local/go/src/runtime/proc.go:238 +0xcc fp=0xc42003e7e0 sp=0xc42003e7a8 pc=0x424e5c\n
runtime.goexit()\n
	/usr/local/go/src/runtime/asm_amd64.s:2337 +0x1 fp=0xc42003e7e8 sp=0xc42003e7e0 pc=0x44b4d1\n
created by runtime.init.4\n
	/usr/local/go/src/runtime/proc.go:227 +0x35\n
\n
goroutine 3 [GC sweep wait]:\n
runtime.gopark(0x4739b8, 0x4ad7e0, 0x46fdd2, 0xd, 0x419914, 0x1)\n
	/usr/local/go/src/runtime/proc.go:280 +0x12c fp=0xc42003ef60 sp=0xc42003ef30 pc=0x42503c\n
runtime.goparkunlock(0x4ad7e0, 0x46fdd2, 0xd, 0x14, 0x1)\n
	/usr/local/go/src/runtime/proc.go:286 +0x5e fp=0xc42003efa0 sp=0xc42003ef60 pc=0x42512e\n
runtime.bgsweep(0xc42001e150)\n
	/usr/local/go/src/runtime/mgcsweep.go:52 +0xa3 fp=0xc42003efd8 sp=0xc42003efa0 pc=0x419973\n
runtime.goexit()\n
	/usr/local/go/src/runtime/asm_amd64.s:2337 +0x1 fp=0xc42003efe0 sp=0xc42003efd8 pc=0x44b4d1\n
created by runtime.gcenable\n
	/usr/local/go/src/runtime/mgc.go:216 +0x58\n",
            // second line
            "one more line, no multiline\n",
        ];
    }
}
