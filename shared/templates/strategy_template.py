import talib.abstract as ta
import pandas as pd
from functools import reduce
from pandas import DataFrame
from freqtrade.strategy import IStrategy

class {{ strategy_name }}(IStrategy):
    INTERFACE_VERSION: int = 3

    minimal_roi = {
        "60": {{ minimal_roi_60 }},
        "30": {{ minimal_roi_30 }},
        "0": {{ minimal_roi_0 }}
    }

    stoploss = {{ stoploss }}

    trailing_stop = {% if trailing_stop %}True{% else %}False{% endif %}
    trailing_stop_positive = {{ trailing_stop_positive }}
    trailing_stop_positive_offset = {{ trailing_stop_offset }}
    trailing_only_offset_is_reached = True

    timeframe = '{{ timeframe }}'

    startup_candle_count: int = {{ startup_candle_count }}

    def informative_pairs(self):
        return []

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
{% if use_rsi %}
        dataframe['rsi'] = ta.RSI(dataframe, period={{ rsi_period }})
{% endif %}
        
{% if use_macd %}
        macd = ta.MACD(dataframe, fastperiod={{ macd_fast }}, slowperiod={{ macd_slow }}, signalperiod={{ macd_signal }})
        dataframe['macd'] = macd['macd']
        dataframe['macdsignal'] = macd['macdsignal']
        dataframe['macdhist'] = macd['macdhist']
{% endif %}

{% if use_ema %}
        dataframe['ema_fast'] = ta.EMA(dataframe, timeperiod={{ ema_fast }})
        dataframe['ema_slow'] = ta.EMA(dataframe, timeperiod={{ ema_slow }})
{% endif %}

{% if use_bb %}
        bollinger = ta.BBANDS(dataframe, timeperiod={{ bb_period }}, nbdevup=2, nbdevdn=2)
        dataframe['bb_upper'] = bollinger['upperband']
        dataframe['bb_middle'] = bollinger['middleband']
        dataframe['bb_lower'] = bollinger['lowerband']
        dataframe['bb_percent'] = (dataframe['close'] - dataframe['bb_lower']) / (dataframe['bb_upper'] - dataframe['bb_lower'])
{% endif %}

{% if use_stochastic %}
        stochastic = ta.STOCH(dataframe, fastk_period={{ stochastic_period }}, slowk_period={{ stochastic_smooth_k }}, slowd_period={{ stochastic_smooth_d }})
        dataframe['stoch_k'] = stochastic['slowk']
        dataframe['stoch_d'] = stochastic['slowd']
{% endif %}

{% if use_adx %}
        dataframe['adx'] = ta.ADX(dataframe, timeperiod={{ adx_period }})
{% endif %}

        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        conditions = []
        
{% if entry_condition_rsi %}
        conditions.append(dataframe['rsi'] < {{ rsi_oversold }})
{% endif %}

{% if entry_condition_macd %}
        conditions.append((dataframe['macd'] > dataframe['macdsignal']) | (dataframe['macdhist'] > 0))
{% endif %}

{% if entry_condition_ema %}
        conditions.append(dataframe['ema_fast'] > dataframe['ema_slow'])
{% endif %}

{% if entry_condition_bb %}
        conditions.append(dataframe['bb_percent'] < 0.2)
{% endif %}

{% if entry_condition_stochastic %}
        conditions.append(dataframe['stoch_k'] < {{ stochastic_oversold }})
{% endif %}

{% if entry_condition_adx %}
        conditions.append(dataframe['adx'] > {{ adx_threshold }})
{% endif %}

        if conditions:
            dataframe.loc[
                reduce(lambda x, y: x & y, conditions),
                'enter_long'
            ] = 1

        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
{% if exit_condition_rsi %}
        dataframe.loc[
            (dataframe['rsi'] > {{ rsi_overbought }}),
            'exit_long'
        ] = 1
{% endif %}

{% if exit_condition_stochastic %}
        dataframe.loc[
            (dataframe['stoch_k'] > {{ stochastic_overbought }}),
            'exit_long'
        ] = 1
{% endif %}

        return dataframe

