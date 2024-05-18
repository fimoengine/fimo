from ..context import Context


def test_context_init():
    ctx = Context.new_context()
    ctx.check_version()
    ctx2 = ctx.acquire()
    del ctx2
