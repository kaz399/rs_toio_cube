# パワーポイントコントローラー cubekey

これはtoioWindows10のキーイベントを発生させてパワーポイントを操作するためのプログラムです

## 必要なもの

toioコアキューブ（バージョンアップ版）

## 準備

Windows10 PC 側と toioコアキューブ（以下キューブと記載します）をペアリングしてください

## 使い方

### 起動方法

1. キューブの電源を入れます
2. cubekey.exeを実行します
3. しばらく待つとキューブが接続されます（キューブのランプが暗い緑色点灯になります）
4. パワーポイントの資料を開きます
5. ウィンドウのフォーカスをパワーポイントに合わせてください（パワーポイントが画面の最前面にある状態にしてください）

### キューブ操作

* キューブをひっくり返した状態でボタン押し → ページが進む（PageDownキー）
* キューブを横倒しにした状態でボタン押し → ページが 戻る（PageUpキー）
* キューブのボタンを長押し（1.5秒以上） → 最初のスライドに戻る（Homeキー）
* キューブのボタンをダブルクリック → プレゼンテーション開始（F5キー）

ダブルクリックはゆっくりめに操作するのがコツです  
素早い操作は認識できません

以下はオマケ機能です

* キューブをダブルタップ → プレゼンテーション開始 ＋ ファンファーレが流れる ＋ キューブ回転

### 終了方法

1. cubekey.exeを停止します（cubekeyを実行したコマンドプロンプトの窓で、Ctrl+Cを押します）
2. キューブのランプが消えたことを確認します
3. キューブの電源を切ります

#### 注意

キューブの電源を切った後にcubekey.exeを終了しようとすると、cubekey.exeがなかなか終了しません
（終了処理でキューブと通信を行うため、キューブが切断されていると通信失敗判定に時間がかかります）

cubekey.exeのウィンドウを×ボタンで消すと終了処理ができなくなり、cuekey.exeがしばらく起動できなくなる現象（起動してもすぐウィンドウが消えてキューブと接続できない）が起きます。
この場合、キューブを再起動（電源オフ→電源オン）してからcubekey.exeの起動を繰り返してください。
そのうち接続可能な状態になるはずです。



## メモなど

cubekey.exeの実行途中でキューブが切断されてしまった場合には、cubekey.exeを再起動してください  
(cubekey.exeは再接続の機能が実装されていません）

キューブおよびcubekey.exeの起動直後は動作が不安定なことがあります  
プレゼンに使う場合は、あらかじめ余裕を持ってキューブとcubekey.exeを起動しておくことをおすすめします