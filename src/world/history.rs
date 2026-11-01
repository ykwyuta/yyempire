//! 史実イベント：英雄・諸子百家・政治指導者の登場、反乱・簒奪による君主家（国姓）の交代
//!
//! 年表は紀元前の年で並べ、その年が来たら発生する。対象国が滅亡（または未成立）していれば省略する。

use super::*;

const ZHOU: usize = 0;
const QIN: usize = 2;
const CHU: usize = 3;
const QI: usize = 4;
const YAN: usize = 5;
const LU: usize = 6;
const SONG: usize = 7;
/// 衛
const EI: usize = 8;
const ZHENG: usize = 9;
const CHEN: usize = 10;
const WU: usize = 14;
const YUE: usize = 15;
const ZHONGSHAN: usize = 16;

/// 各国の建国時の君主家（国姓）。NATIONS と同じ並び
pub const INITIAL_HOUSE: &[&str] = &[
    "姫姓", "姫姓（翼系）", "嬴姓趙氏", "羋姓熊氏", "姜姓呂氏", "姫姓", "姫姓", "子姓", "姫姓", "姫姓", "嬀姓", "姫姓", "姫姓", "姜姓", "姫姓", "姒姓", "姫姓（鮮虞）",
    "姫姓", "姫姓", "曼姓", "姜姓", "姫姓", "姫姓", "開明氏", "嬴姓", "己姓", "任姓", "姫姓", "隗姓", "義渠戎王", "庸伯", "偃姓", "嬴姓", "墨胎氏", "姜姓", "己姓",
    "代君", "楼煩王", "姫姓韓氏", "嬴姓趙氏", "姫姓魏氏",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HKind {
    /// 英雄・名将・名君
    Hero,
    /// 諸子百家
    Sage,
    /// 政治指導者（宰相・改革者）
    Minister,
    /// 反乱・内乱
    Revolt,
    /// 簒奪・乗っ取り（君主家の交代）
    Usurp,
    /// 旧君主家の復辟
    Restore,
    /// 大敗・災厄
    Disaster,
}

impl HKind {
    pub fn label(self) -> &'static str {
        match self {
            HKind::Hero => "英雄",
            HKind::Sage => "諸子百家",
            HKind::Minister => "政治指導者",
            HKind::Revolt => "反乱",
            HKind::Usurp => "簒奪",
            HKind::Restore => "復辟",
            HKind::Disaster => "大敗",
        }
    }
    /// 国にとって不利な出来事か
    pub fn bad(self) -> bool {
        matches!(self, HKind::Revolt | HKind::Usurp | HKind::Disaster)
    }
    pub fn color(self) -> [f32; 3] {
        if self.bad() {
            [1.0, 0.55, 0.45]
        } else {
            [0.75, 0.9, 1.0]
        }
    }
}

/// イベントの効果（対象は原則としてイベントの国）
#[derive(Clone, Copy, Debug)]
pub enum Fx {
    /// 人物が仕える：生産（税・採集）と兵の攻撃力を一定年数だけ高める（負なら混乱）
    Figure(&'static str, f32, f32, f32),
    /// 人物が去る
    Dismiss(&'static str),
    /// 資源を得る
    Grant(f32, f32, f32),
    /// 全兵の能力を上げる（時代上限まで）
    Train(f32),
    /// 都に戈兵を召集
    Levy(u32),
    /// 兵の一部が離反・戦死する
    Purge(f32),
    /// 蓄えの一部を失う
    Plunder(f32),
    /// 君主家（国姓）が変わる
    House(&'static str),
    /// 指定国を標的とした合従軍
    Hezong(usize),
    /// 自国を標的とする連合を解散させる（連衡）
    Breakup,
    /// a と b が休戦する
    Truce(usize, usize, f32),
    /// a が b に宣戦する
    War(usize, usize),
}

pub struct HEvent {
    pub bc: i32,
    pub nation: usize,
    pub kind: HKind,
    pub title: &'static str,
    pub text: &'static str,
    pub fx: &'static [Fx],
}

macro_rules! ev {
    ($bc:expr, $n:expr, $k:ident, $t:expr, $x:expr, [$($fx:expr),* $(,)?]) => {
        HEvent { bc: $bc, nation: $n, kind: HKind::$k, title: $t, text: $x, fx: &[$($fx),*] }
    };
}

use Fx::*;

/// 史実年表（紀元前の年の降順）
pub const HISTORY: &[HEvent] = &[
    // ---------------------------------------------------------------- 春秋
    ev!(743, ZHENG, Hero, "鄭の荘公、即位", "鄭の荘公が即位した。周の卿士として王をも凌ぎ「春秋の小覇」と称される。", [Figure("鄭荘公", 0.25, 0.25, 42.0), Grant(150.0, 100.0, 60.0)]),
    ev!(719, EI, Usurp, "州吁の乱", "衛の公子州吁が兄の桓公を弑して自ら君位に就いた。", [House("姫姓（州吁）"), Purge(0.3), Plunder(0.3)]),
    ev!(718, EI, Restore, "大義、親を滅す", "老臣石碏が我が子もろとも州吁を討ち、宣公を迎えて衛の公室を立て直した。", [House("姫姓"), Figure("石碏", 0.1, 0.1, 10.0)]),
    ev!(704, CHU, Hero, "楚の武王、王を称す", "楚の熊通が自ら「王」を称し、漢水流域の姫姓諸国を圧迫し始めた。", [Figure("楚武王", 0.15, 0.3, 14.0), Levy(3)]),
    ev!(686, QI, Revolt, "公孫無知の乱", "斉の公孫無知が襄公を弑して自立したが、翌年には討たれた。", [Purge(0.3), Plunder(0.4)]),
    ev!(685, QI, Minister, "管仲、斉の宰相となる", "桓公は仇敵であった管仲を宰相に迎えた。「倉廩実ちて礼節を知る」──富国強兵が始まる。", [Figure("管仲", 0.5, 0.2, 40.0), Grant(300.0, 150.0, 100.0)]),
    ev!(679, QI, Hero, "斉の桓公、覇者となる", "斉の桓公が鄄で諸侯と会盟し、春秋最初の覇者となった。", [Figure("斉桓公", 0.0, 0.3, 36.0), Train(0.5)]),
    ev!(678, JIN, Usurp, "曲沃、翼に代わる", "分家の曲沃武公が本家の晋侯緡を滅ぼし、周王から晋侯に封じられた。晋の君統は曲沃系に移る。", [House("姫姓（曲沃系）"), Purge(0.25), Figure("曲沃武公", 0.2, 0.2, 10.0)]),
    ev!(656, JIN, Revolt, "驪姫の乱", "献公の寵姫驪姫の讒言で太子申生が自害し、公子重耳・夷吾は国外へ逃れた。", [Purge(0.2), Figure("驪姫の乱", -0.2, 0.0, 8.0)]),
    ev!(655, QIN, Minister, "五羖大夫・百里奚", "秦の穆公が五枚の羊皮で百里奚を買い戻し、国政を任せた。", [Figure("百里奚", 0.35, 0.1, 30.0), Grant(200.0, 150.0, 50.0)]),
    ev!(636, JIN, Hero, "重耳、帰国す", "十九年の亡命を経て重耳が帰国し晋の文公となった。城濮で楚を破り覇者となる。", [Figure("晋文公", 0.25, 0.35, 15.0), Levy(4), Train(0.5)]),
    ev!(623, QIN, Minister, "由余、西戎を制す", "戎出身の由余の策により、秦の穆公は西戎十二国を併せて西戎の覇者となった。", [Figure("由余", 0.15, 0.3, 20.0), Grant(150.0, 100.0, 80.0)]),
    ev!(613, CHU, Hero, "鳴かず飛ばず", "「三年鳴かず飛ばず」の楚の荘王が覚醒した。鼎の軽重を問い、邲で晋を破る。", [Figure("楚荘王", 0.3, 0.35, 22.0), Train(0.5)]),
    ev!(605, CHU, Revolt, "若敖氏の乱", "楚の令尹闘越椒が若敖氏の一族を率いて叛き、荘王に鎮圧された。", [Purge(0.25), Plunder(0.25)]),
    ev!(601, CHU, Minister, "孫叔敖、令尹となる", "令尹孫叔敖が芍陂を築いて灌漑を整え、楚の農業を大いに興した。", [Figure("孫叔敖", 0.4, 0.0, 25.0)]),
    ev!(599, CHEN, Usurp, "夏徴舒の乱", "陳の大夫夏徴舒が霊公を射殺し、自ら陳侯を称した。", [House("嬀姓夏氏"), Purge(0.3), Plunder(0.3)]),
    ev!(598, CHEN, Restore, "楚、陳を正す", "楚の荘王が陳に攻め入って夏徴舒を討ち、霊公の子を君に立てた。", [House("嬀姓")]),
    ev!(584, WU, Hero, "巫臣、呉に車戦を伝う", "晋から遣わされた申公巫臣が呉に車戦と弓術を伝えた。呉の寿夢は王を称し楚を脅かす。", [Figure("申公巫臣", 0.1, 0.35, 30.0), Train(0.5)]),
    ev!(548, QI, Revolt, "崔杼、その君を弑す", "権臣崔杼が荘公を弑した。太史の兄弟は命を賭して「崔杼、その君を弑す」と書き続けた。", [Purge(0.25), Plunder(0.3)]),
    ev!(547, QI, Minister, "晏嬰、宰相となる", "晏嬰が景公の宰相となり、倹約と直言で斉を支えた。", [Figure("晏嬰", 0.3, 0.0, 47.0)]),
    ev!(546, SONG, Minister, "弭兵の会", "宋の向戌の斡旋で、晋・楚をはじめ十四国が宋に会し停戦を誓った。", [Figure("向戌", 0.2, 0.0, 15.0), Truce(JIN, CHU, 20.0)]),
    ev!(543, ZHENG, Minister, "子産、鄭の執政となる", "子産が鄭の執政となり、田制を整え刑書を鼎に鋳て、大国の間で鄭を保った。", [Figure("子産", 0.45, 0.1, 21.0)]),
    ev!(537, LU, Revolt, "三桓、公室を四分す", "季孫・叔孫・孟孫の三桓が公室の軍と領地を分け取り、魯君は実権を失った。", [Plunder(0.3), Figure("三桓専権", -0.1, 0.0, 20.0)]),
    ev!(522, WU, Hero, "伍子胥、呉に奔る", "楚の平王に父兄を殺された伍子胥が呉に亡命し、復讐を誓った。", [Figure("伍子胥", 0.25, 0.25, 38.0)]),
    ev!(520, ZHOU, Revolt, "王子朝の乱", "景王の死後に王子朝が王位を争い、王城は十数年にわたって二つに割れた。", [Purge(0.3), Plunder(0.4), Figure("王子朝の乱", -0.25, 0.0, 15.0)]),
    ev!(515, WU, Usurp, "専諸、王僚を刺す", "公子光が刺客専諸に呉王僚を刺殺させ、自ら即位した（呉王闔閭）。", [House("姫姓（闔閭系）"), Purge(0.15), Figure("呉王闔閭", 0.2, 0.3, 19.0)]),
    ev!(512, WU, Sage, "兵家・孫武", "『孫子』の兵法家孫武が闔閭に仕え、呉軍を鍛え上げた。", [Figure("孫武", 0.0, 0.4, 20.0), Train(0.5)]),
    ev!(506, WU, Hero, "柏挙の戦い", "呉王闔閭が孫武・伍子胥とともに楚に攻め込み、楚都郢を陥とした。", [War(WU, CHU), Levy(3)]),
    ev!(505, CHU, Hero, "申包胥、秦庭に哭す", "楚の申包胥が秦の宮廷で七日七晩泣き続け、秦の援軍を得て楚を救った。", [Levy(4), Truce(QIN, CHU, 20.0), Truce(WU, CHU, 10.0)]),
    ev!(500, LU, Sage, "孔子、大司寇となる", "儒家の祖・孔子が魯の大司寇となり、夾谷の会で斉から失地を取り戻した。", [Figure("孔子と門弟", 0.25, 0.0, 25.0), Grant(100.0, 100.0, 50.0)]),
    ev!(497, JIN, Revolt, "范氏・中行氏の乱", "晋の六卿のうち范氏・中行氏が趙氏と争い、晋は長い内乱に陥った。", [Purge(0.25), Figure("六卿の争い", -0.2, 0.0, 8.0)]),
    ev!(494, YUE, Minister, "臥薪嘗胆", "会稽で呉に敗れた越王勾践は、范蠡・文種とともに雪辱を期して国力を蓄えた。", [Figure("范蠡・文種", 0.5, 0.1, 25.0)]),
    ev!(484, WU, Disaster, "伍子胥、死を賜る", "讒言を信じた呉王夫差が伍子胥に自害を命じた。「我が目を東門に懸けよ」。", [Dismiss("伍子胥"), Figure("夫差の驕り", -0.15, -0.1, 10.0)]),
    ev!(482, YUE, Hero, "越王勾践、呉を討つ", "夫差が黄池の会に出た隙を突き、越王勾践が呉に攻め込んだ。", [Figure("越王勾践", 0.1, 0.35, 20.0), Train(0.5), War(YUE, WU)]),
    ev!(481, QI, Revolt, "田常、簡公を弑す", "田常が簡公を弑して実権を握った。以後、斉の政は田氏の手に移る。", [House("姜姓呂氏（実権は田氏）"), Purge(0.2), Figure("田常", 0.25, 0.0, 20.0)]),
    ev!(479, CHU, Revolt, "白公勝の乱", "楚の白公勝が令尹子西を殺して叛いたが、葉公子高に討たれた。", [Purge(0.25), Plunder(0.25)]),
    // ---------------------------------------------------------------- 戦国
    ev!(445, WEI, Hero, "魏の文侯", "魏の文侯が即位し、李悝・呉起・西門豹・楽羊ら天下の賢才を集めた。", [Figure("魏文侯", 0.2, 0.2, 50.0)]),
    ev!(440, SONG, Sage, "墨子、宋を守る", "墨家の墨子が楚王の前で公輸盤の攻城具を九度防ぎ、楚の宋攻めを止めさせた。", [Figure("墨家の守城", 0.0, 0.35, 30.0), Truce(SONG, CHU, 20.0)]),
    ev!(422, WEI, Minister, "李悝の変法", "李悝が『法経』を著し、地力を尽くす農政で魏を戦国最初の強国とした。", [Figure("李悝", 0.5, 0.0, 40.0), Grant(250.0, 150.0, 100.0)]),
    ev!(409, WEI, Hero, "呉起、西河の守となる", "名将呉起が西河の守となり、秦から河西の地を奪った。", [Figure("呉起", 0.0, 0.4, 25.0), Train(0.5), Levy(3)]),
    ev!(406, ZHONGSHAN, Usurp, "楽羊、中山を降す", "魏の楽羊が中山を攻め落とし、魏の公子撃が中山君に封じられた。", [House("姫姓魏氏（魏の公子）"), Purge(0.5), Plunder(0.3), Truce(ZHONGSHAN, WEI, 25.0)]),
    ev!(397, HAN, Revolt, "聶政、侠累を刺す", "刺客聶政が韓の宰相侠累を刺殺した。", [Plunder(0.2), Purge(0.1)]),
    ev!(386, QI, Usurp, "田氏、斉を代わる", "田和が康公を海浜に遷し、周王から斉侯に封じられた。姜姓の斉は絶え、田斉となる。", [House("嬀姓田氏"), Purge(0.15), Figure("斉太公田和", 0.2, 0.1, 10.0)]),
    ev!(385, QIN, Restore, "秦の献公、帰国す", "亡命先の魏から献公が帰国して即位し、殉死を廃して戸籍を整えた。", [Figure("秦献公", 0.25, 0.15, 23.0)]),
    ev!(382, CHU, Minister, "呉起の変法", "魏を追われた呉起が楚の令尹となり、世襲貴族の特権を削る変法を断行した。", [Figure("呉起の変法", 0.35, 0.25, 20.0)]),
    ev!(381, CHU, Revolt, "呉起の死", "悼王の死とともに旧貴族が叛き、呉起は王の遺体に伏したまま射殺された。", [Dismiss("呉起の変法"), Purge(0.15)]),
    ev!(380, ZHONGSHAN, Restore, "中山の復国", "中山の桓公が魏の支配を脱し、鮮虞の中山国を復興した。", [House("姫姓（鮮虞）"), Figure("中山桓公", 0.2, 0.2, 30.0)]),
    ev!(367, ZHOU, Revolt, "東西周の分裂", "周王室は東周と西周に分裂し、王畿はさらに細った。", [Purge(0.3), Plunder(0.3), Figure("東西周の分裂", -0.3, 0.0, 100.0)]),
    ev!(359, QIN, Minister, "商鞅の変法", "孝公が衛の商鞅を登用した。什伍の制・軍功爵・県制を敷き、秦は最強国への道を歩み出す。", [Figure("商鞅の法", 0.6, 0.3, 140.0), Train(0.5), Grant(300.0, 200.0, 150.0)]),
    ev!(356, QI, Hero, "一鳴、人を驚かす", "斉の威王が即墨大夫を賞し阿大夫を烹て、鄒忌を宰相に国政を改めた。", [Figure("斉威王・鄒忌", 0.35, 0.2, 36.0)]),
    ev!(356, SONG, Usurp, "戴氏、宋を取る", "宋の大臣戴氏（剔成君）が宋公を廃して君位を奪った。", [House("子姓戴氏"), Purge(0.2), Plunder(0.2)]),
    ev!(355, HAN, Minister, "申不害の術", "韓の昭侯が法家の申不害を宰相とし、「術」による統治で国内を引き締めた。", [Figure("申不害", 0.4, 0.15, 18.0)]),
    ev!(353, QI, Sage, "兵家・孫臏", "孫臏が「魏を囲んで趙を救う」策で桂陵に魏軍を破った。", [Figure("孫臏", 0.0, 0.45, 20.0), Train(0.5)]),
    ev!(341, WEI, Disaster, "馬陵の戦い", "孫臏の計に陥った魏軍は馬陵で全滅し、龐涓は自害、太子申は捕らわれた。", [Purge(0.4), Figure("馬陵の敗戦", -0.1, -0.1, 10.0)]),
    ev!(338, QIN, Revolt, "商鞅、車裂きとなる", "孝公の死後、商鞅は反逆の罪で車裂きにされた。しかし秦は彼の法を守り続けた。", [Purge(0.1)]),
    ev!(333, YAN, Sage, "蘇秦の合従", "縦横家の蘇秦が六国を説いて回り、秦に対する合従を成立させた。", [Figure("蘇秦", 0.1, 0.1, 15.0), Hezong(QIN)]),
    ev!(328, QIN, Sage, "張儀の連衡", "縦横家の張儀が秦の宰相となり、連衡策で六国の合従を切り崩した。", [Figure("張儀", 0.2, 0.1, 18.0), Breakup]),
    ev!(319, QI, Sage, "稷下の学", "斉の宣王が稷下に学者を集め、孟子・荀子・鄒衍らが百家の論を競った。", [Figure("稷下の学士", 0.35, 0.0, 50.0)]),
    ev!(318, SONG, Hero, "宋王偃", "宋の君偃が王を称し、東は斉、南は楚、西は魏を攻めた。", [Figure("宋王偃", 0.0, 0.3, 30.0), Levy(3)]),
    ev!(316, YAN, Usurp, "子之の乱", "燕王噲が宰相子之に王位を譲った。太子平と国人が叛き、燕は大乱に陥る。", [House("子之（宰相）"), Purge(0.35), Plunder(0.4), Figure("子之の乱", -0.35, -0.1, 4.0)]),
    ev!(314, QI, Hero, "匡章、燕を討つ", "斉の宣王が燕の内乱に乗じ、将軍匡章に燕を攻めさせた。", [Figure("匡章", 0.0, 0.3, 10.0), War(QI, YAN)]),
    ev!(312, YAN, Restore, "まず隗より始めよ", "燕の昭王が即位して姫姓の王統を回復した。郭隗を厚遇し、天下の賢士を招く。", [House("姫姓"), Figure("郭隗・燕昭王", 0.35, 0.15, 33.0), Truce(QI, YAN, 10.0)]),
    ev!(307, ZHAO, Hero, "胡服騎射", "趙の武霊王が胡服騎射を採用し、騎兵を育てて中山・林胡・楼煩を攻めた。", [Figure("胡服騎射", 0.1, 0.45, 40.0), Train(0.5), Levy(3)]),
    ev!(298, QI, Minister, "孟嘗君", "斉の孟嘗君が食客三千を養い、韓・魏とともに函谷関を攻めた。", [Figure("孟嘗君", 0.2, 0.2, 20.0)]),
    ev!(295, ZHAO, Revolt, "沙丘の乱", "主父（武霊王）が公子章の乱に巻き込まれ、沙丘宮で餓死した。", [Purge(0.3), Plunder(0.3)]),
    ev!(284, YAN, Hero, "楽毅、斉を破る", "燕の楽毅が五国の連合軍を率いて済西で斉を破り、七十余城を陥とした。", [Figure("楽毅", 0.0, 0.45, 5.0), Train(0.5), Hezong(QI)]),
    ev!(279, QI, Hero, "田単の火牛", "即墨を守る田単が火牛の計で燕軍を破り、斉の七十余城を奪い返した。", [Figure("田単", 0.1, 0.4, 15.0), Levy(5)]),
    ev!(279, ZHAO, Minister, "刎頸の交わり", "完璧帰趙の藺相如と名将廉頗が刎頸の交わりを結び、趙を支えた。", [Figure("藺相如・廉頗", 0.2, 0.3, 30.0)]),
    ev!(278, CHU, Disaster, "郢都陥落", "秦将白起が楚都郢を陥とした。憂国の詩人屈原は汨羅に身を投げる。", [Purge(0.35), Plunder(0.3)]),
    ev!(270, QIN, Minister, "遠交近攻", "范雎の「遠交近攻」策を昭王が採用した。遠い斉・燕と結び、近い韓・魏を攻める。", [Figure("范雎", 0.2, 0.2, 15.0), Truce(QIN, QI, 20.0), Truce(QIN, YAN, 20.0)]),
    ev!(260, ZHAO, Disaster, "長平の戦い", "紙上に兵を談ずる趙括が長平で大敗し、趙は四十万の兵を失った。", [Purge(0.6), Dismiss("藺相如・廉頗")]),
    ev!(260, QIN, Hero, "白起", "長平で趙軍を破った秦将白起の名は天下を震わせた。", [Figure("白起", 0.0, 0.45, 3.0), Train(0.5)]),
    ev!(257, WEI, Hero, "信陵君、趙を救う", "魏の信陵君が兵符を盗み、邯鄲を包囲する秦軍を破って趙を救った。", [Figure("信陵君", 0.1, 0.4, 15.0), Levy(4)]),
    ev!(249, QIN, Minister, "呂不韋、相国となる", "大商人呂不韋が秦の相国となり、食客三千を集めて『呂氏春秋』を編んだ。", [Figure("呂不韋", 0.35, 0.1, 15.0)]),
    ev!(247, WEI, Hero, "五国合従", "信陵君が五国の兵を率い、河外で秦の蒙驁を破った。", [Hezong(QIN)]),
    ev!(241, CHU, Minister, "春申君の合従", "楚の春申君が最後の合従軍を率いて函谷関に迫った。", [Figure("春申君", 0.2, 0.1, 3.0), Hezong(QIN)]),
    ev!(238, QIN, Revolt, "嫪毐の乱", "嫪毐が太后の印璽を使って兵を挙げたが、秦王政に鎮圧された。", [Purge(0.15)]),
    ev!(237, QIN, Minister, "李斯、逐客を諫む", "李斯が「諫逐客書」を奉って秦王政に重用され、呂不韋は失脚した。", [Dismiss("呂不韋"), Figure("李斯", 0.3, 0.2, 30.0)]),
    ev!(233, QIN, Sage, "韓非の法術", "韓の公子韓非が秦に使いし獄死したが、その法術の思想は秦王政に採り入れられた。", [Figure("韓非の法術", 0.15, 0.1, 30.0)]),
    ev!(233, ZHAO, Hero, "李牧", "趙の名将李牧が肥で秦軍を破った。", [Figure("李牧", 0.0, 0.4, 5.0), Levy(3)]),
    ev!(229, ZHAO, Disaster, "李牧誅殺", "秦の反間に惑わされた趙王遷が、名将李牧を誅殺した。", [Dismiss("李牧"), Purge(0.2)]),
    ev!(227, YAN, Hero, "風蕭蕭として易水寒し", "燕の太子丹が刺客荊軻を秦王政のもとへ送ったが、暗殺は失敗した。", [War(QIN, YAN)]),
    ev!(224, QIN, Hero, "王翦", "老将王翦が六十万の兵を率いて楚に攻め込んだ。", [Figure("王翦", 0.0, 0.45, 10.0), Levy(5), War(QIN, CHU)]),
];

/// 在世の人物（効果は年限つき）
pub struct Person {
    pub name: &'static str,
    pub eco: f32,
    pub mil: f32,
    pub until: f32,
}

/// 史書の記録
pub struct Chron {
    pub bc: i32,
    pub nation: usize,
    pub kind: HKind,
    pub title: &'static str,
    pub text: &'static str,
    /// 対象国が滅んでいて起こらなかった
    pub skipped: bool,
}

impl World {
    /// 年が改まるごとに史実イベントを発生させる
    pub(super) fn run_history(&mut self) {
        let yb = self.year_bc();
        // 人物の退場
        let years = self.years;
        for n in self.nations.iter_mut() {
            n.figures.retain(|f| f.until > years);
        }
        while self.hist_i < HISTORY.len() && HISTORY[self.hist_i].bc >= yb {
            let e = &HISTORY[self.hist_i];
            self.hist_i += 1;
            self.fire_event(e);
        }
        self.refresh_mods();
    }

    fn fire_event(&mut self, e: &'static HEvent) {
        let n = e.nation;
        let skipped = !self.nations[n].alive;
        self.chronicle.push(Chron { bc: e.bc, nation: n, kind: e.kind, title: e.title, text: e.text, skipped });
        if skipped {
            return;
        }
        // チュートリアル中はプレイヤーに不利な出来事を起こさない
        let shield = self.protect_player && n == self.player;
        let mut notes: Vec<String> = Vec::new();
        for fx in e.fx.iter() {
            match *fx {
                Figure(name, eco, mil, yrs) => {
                    if shield && (eco < 0.0 || mil < 0.0) {
                        continue;
                    }
                    let until = self.years + yrs;
                    let nat = &mut self.nations[n];
                    nat.figures.retain(|f| f.name != name);
                    nat.figures.push(Person { name, eco, mil, until });
                    let mut s = String::from(name);
                    if eco != 0.0 {
                        s.push_str(&format!(" 生産{:+.0}%", eco * 100.0));
                    }
                    if mil != 0.0 {
                        s.push_str(&format!(" 武{:+.0}%", mil * 100.0));
                    }
                    s.push_str(&format!("（{}年）", yrs.min((START_BC - END_BC) as f32) as i32));
                    notes.push(s);
                }
                Dismiss(name) => {
                    let nat = &mut self.nations[n];
                    if nat.figures.iter().any(|f| f.name == name) {
                        nat.figures.retain(|f| f.name != name);
                        notes.push(format!("{} が去る", name));
                    }
                }
                Grant(a, b, c) => {
                    let r = &mut self.nations[n].res;
                    r[0] += a;
                    r[1] += b;
                    r[2] += c;
                    notes.push(format!("糧{:.0} 木{:.0} 銅{:.0}", a, b, c));
                }
                Train(x) => {
                    let cap = self.power_cap();
                    let mut k = 0;
                    for u in self.units.iter_mut() {
                        if u.alive && u.nation as usize == n && u.kind.military() && u.power < cap {
                            let ratio = u.hp / u.max_hp();
                            u.power = (u.power + x).min(cap);
                            u.hp = ratio * u.max_hp();
                            k += 1;
                        }
                    }
                    if k > 0 {
                        notes.push(format!("兵{}人の能力{:+.1}", k, x));
                    }
                }
                Levy(cnt) => {
                    let Some(c) = self.nations[n].capital else { continue };
                    let pw = (1.0 + self.years / (START_BC - END_BC) * 2.0).min(self.power_cap());
                    let mut k = 0;
                    for _ in 0..cnt {
                        if let Some(sp) = self.spawn_point(c) {
                            self.spawn_unit(n, UnitKind::Infantry, sp, pw);
                            k += 1;
                        }
                    }
                    if k > 0 {
                        notes.push(format!("戈兵{}人が馳せ参じる", k));
                    }
                }
                Purge(f) => {
                    if shield {
                        continue;
                    }
                    let mut idx: Vec<usize> = (0..self.units.len()).filter(|&i| self.units[i].alive && self.units[i].nation as usize == n && self.units[i].kind.military()).collect();
                    // 都から遠い兵ほど離反しやすい
                    let home = self.nations[n].home;
                    idx.sort_by(|&a, &b| self.units[b].pos.dist(home).partial_cmp(&self.units[a].pos.dist(home)).unwrap());
                    let k = (idx.len() as f32 * f).round() as usize;
                    for &i in idx.iter().take(k) {
                        self.kill_unit(i);
                    }
                    if k > 0 {
                        notes.push(format!("兵{}人を失う", k));
                    }
                }
                Plunder(f) => {
                    if shield {
                        continue;
                    }
                    for r in self.nations[n].res.iter_mut() {
                        *r *= 1.0 - f;
                    }
                    notes.push(format!("蓄えの{:.0}%を失う", f * 100.0));
                }
                House(h) => {
                    if self.nations[n].house != h {
                        notes.push(format!("君主家：{} → {}", self.nations[n].house, h));
                        self.nations[n].house = h;
                    }
                }
                Hezong(t) => {
                    if self.nations[t].alive && !self.coalitions.iter().any(|c| c.target == t) && self.start_coalition_ex(CoalKind::Hezong, t, false) {
                        let c = self.coalitions.last().unwrap();
                        notes.push(format!("{}か国の合従軍が{}を攻める（盟主{}）", c.members.len(), self.nations[t].name, self.nations[c.leader].name));
                    }
                }
                Breakup => {
                    let years = self.years;
                    let mut any = false;
                    for c in self.coalitions.iter_mut() {
                        if c.target == n && c.end > years {
                            c.end = years;
                            any = true;
                        }
                    }
                    if any {
                        notes.push(format!("{}を討つ連合が瓦解", self.nations[n].name));
                    }
                }
                Truce(a, b, yrs) => {
                    if !self.nations[a].alive || !self.nations[b].alive || a == b {
                        continue;
                    }
                    if self.coalition_locked(a, b) {
                        continue;
                    }
                    self.make_peace(a, b, yrs);
                    let t = self.years + yrs;
                    self.truce[a][b] = self.truce[a][b].max(t);
                    self.truce[b][a] = self.truce[b][a].max(t);
                    notes.push(format!("{}と{}が{}年の休戦", self.nations[a].name, self.nations[b].name, yrs as i32));
                }
                War(a, b) => {
                    // プレイヤーの国に勝手に宣戦させない
                    if a == self.player || (self.protect_player && b == self.player) {
                        continue;
                    }
                    if self.declare_war(a, b, true) {
                        notes.push(format!("{}が{}に攻め込む", self.nations[a].name, self.nations[b].name));
                    }
                }
            }
        }
        self.refresh_mods();
        self.territory_dirty = true;
        let name = self.nations[n].name;
        let head = format!("【{}・{}】{}", name, e.kind.label(), e.title);
        // 本文はバナーと史記に任せ、記録欄は見出しと効果だけにする
        let mut msg = head;
        if !notes.is_empty() {
            msg.push_str(&format!("（{}）", notes.join("、")));
        }
        self.push_msg(msg, e.kind.color());
        // 大きな出来事はバナーで知らせる（小国の出来事は記録のみ）
        let involves_player = n == self.player
            || e.fx.iter().any(|f| match *f {
                Hezong(t) => t == self.player,
                War(a, b) | Truce(a, b, _) => a == self.player || b == self.player,
                _ => false,
            });
        if involves_player || NATIONS[n].seven {
            let mut text = e.text.to_string();
            if !notes.is_empty() {
                text.push_str(&format!("\n{}", notes.join("　")));
            }
            let title = format!("{}　{}", name, e.title);
            self.queue_banner(Banner { title, text, t: 7.0 });
        }
    }

    /// 在世の人物の効果を集計する
    pub(super) fn refresh_mods(&mut self) {
        let years = self.years;
        for n in self.nations.iter_mut() {
            let (mut e, mut m) = (1.0f32, 1.0f32);
            for f in n.figures.iter().filter(|f| f.until > years) {
                e += f.eco;
                m += f.mil;
            }
            n.eco_mul = e.clamp(0.4, 3.0);
            n.mil_mul = m.clamp(0.5, 2.5);
        }
    }

    pub fn queue_banner(&mut self, b: Banner) {
        if self.banner.is_none() {
            self.banner = Some(b);
        } else {
            self.banner_queue.push_back(b);
        }
    }
}
