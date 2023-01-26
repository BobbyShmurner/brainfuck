	.text
	.def	@feat.00;
	.scl	3;
	.type	0;
	.endef
	.globl	@feat.00
.set @feat.00, 0
	.file	"ticktactoe"
	.def	main;
	.scl	2;
	.type	32;
	.endef
	.globl	main
	.p2align	4, 0x90
main:
.seh_proc main
	pushq	%rsi
	.seh_pushreg %rsi
	subq	$48, %rsp
	.seh_stackalloc 48
	.seh_endprologue
	movl	$30000, %ecx
	callq	malloc
	movq	%rax, %rsi
	movq	%rax, %rcx
	callq	zero_data
	movq	$0, 40(%rsp)
	movzbl	(%rsi), %eax
	movb	%al, 32(%rsp)
	leaq	.L__unnamed_1(%rip), %rcx
	xorl	%edx, %edx
	movq	%rsi, %r8
	movq	%rsi, %r9
	callq	printf
	xorl	%eax, %eax
	addq	$48, %rsp
	popq	%rsi
	retq
	.seh_endproc

	.def	zero_data;
	.scl	2;
	.type	32;
	.endef
	.globl	zero_data
	.p2align	4, 0x90
zero_data:
.seh_proc zero_data
	pushq	%rax
	.seh_stackalloc 8
	.seh_endprologue
	movl	$0, 4(%rsp)
	.p2align	4, 0x90
.LBB1_1:
	movslq	4(%rsp), %rax
	movl	$7, (%rcx,%rax)
	incl	%eax
	movl	%eax, 4(%rsp)
	cmpl	$30000, %eax
	jb	.LBB1_1
	popq	%rax
	retq
	.seh_endproc

	.section	.rdata,"dr"
.L__unnamed_1:
	.asciz	"Index: %d\nData: %p\nOffset: %p\nValue: %d\n"

