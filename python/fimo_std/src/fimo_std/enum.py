from abc import ABCMeta
from enum import EnumMeta


class ABCEnum(ABCMeta, EnumMeta):
    pass
