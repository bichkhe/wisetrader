import talib.abstract as ta
import pandas as pd
from functools import reduce
from pandas import DataFrame
from freqtrade.strategy import IStrategy

class RSI_4h_SOLUSDT_2(IStrategy):
    INTERFACE_VERSION: int = 3

    minimal_roi = {
        "60": 0.05,
        "30": 0.03,
        "0": 0.01
    }

    stoploss = -0.10

    trailing_stop = False
    trailing_stop_positive = 0.02
    trailing_stop_positive_offset = 0.01
    trailing_only_offset_is_reached = True

    timeframe = '4h'

    startup_candle_count: int = 200

    def informative_pairs(self):
        return []

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:

        dataframe['rsi'] = ta.RSI(dataframe, period=14)

        






        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        conditions = []
        

        conditions.append(dataframe['rsi'] < 30)








        if conditions:
            dataframe.loc[
                reduce(lambda x, y: x & y, conditions),
                'enter_long'
            ] = 1

        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:

        return dataframe
