using System.Reflection;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;

namespace Ozi.Clock;

/// <summary>
/// Interaction logic for MainWindow.xaml
/// </summary>
public partial class MainWindow : Window
{
    private List<OsClock> lstClock;
    private double fTransparentValue = 90;
    private bool fIsTransparent = false;
    public bool isAutoFold = false;
    private bool isFolded = false;
    public fmTimeChecker newFmTimeChecker;
    private TimeChecker2 _TimeChecker2;
    private DateTime localTime;
    private Timer timeTimer;
    public bool useSnap = true;
    // private Screen[] screens;

    public MainWindow()
    {
        InitializeComponent();

        newFmTimeChecker = new fmTimeChecker(this);
        _TimeChecker2 = new TimeChecker2(this);
        // screens = Screen.AllScreens;
    }

    public bool isTransparent
    {
        set
        {
            fIsTransparent = value;
            if (!fIsTransparent)
                this.Opacity = 1;
        }
        get { return fIsTransparent; }
    }

    public double transparentValue
    {
        set
        {
            fTransparentValue = value;
            if (fIsTransparent)
                fmMain.Opacity = fTransparentValue;
            else
                fmMain.Opacity = 1;
        }
        get { return fTransparentValue; }
    }

    private void fmMain_Loaded(object sender, RoutedEventArgs e)
    {
        init_Timer();
        read_Config();

        lstClock = new List<OsClock>();

        lstClock.Add((new OsClock("NYK", "Eastern Standard Time", "#FFAAAAFF", lstClock.Count * 100)));
        lstClock.Add((new OsClock("LDN", "GMT Standard Time", "#FFAAFFAA", lstClock.Count * 100)));
        lstClock.Add((new OsClock("KYIV", "FLE Standard Time", "#FFAAFFFF", lstClock.Count * 100)));
        lstClock.Add((new OsClock("PUN", "India Standard Time", "#FF99BBBB", lstClock.Count * 100)));
        lstClock.Add((new OsClock("SGP", "Singapore Standard Time", "#FFFFFFAA", lstClock.Count * 100)));
        lstClock.Add((new OsClock("TKO", "Tokyo Standard Time", "#FFFFAAAA", lstClock.Count * 100)));

        foreach (OsClock item in lstClock)
        {
            gdMain.Children.Add(item.osGrid);
        }

        fmMain.Width = lstClock.Count * 100 + 1;
        lstClock[2].isMain = true;


        //Context menu
        System.Windows.Controls.ContextMenu mainMenu = new System.Windows.Controls.ContextMenu();

        System.Windows.Controls.MenuItem item1 = new System.Windows.Controls.MenuItem();
        item1.Header = "About";
        //item1.Icon = new System.Windows.Controls.Image
        //{
        //    Source = new BitmapImage(new Uri("Resources/Copy16.png", UriKind.Relative))
        //};
        //new BitmapImage(new Uri("Resources/pencil_16.BMP", UriKind.Relative));
        item1.Click += MenuItemAbout_Click;
        mainMenu.Items.Add(item1);

        System.Windows.Controls.MenuItem item3 = new System.Windows.Controls.MenuItem();
        item3.Header = "Settings";
        item3.Click += MenuItemSettings_Click;
        mainMenu.Items.Add(item3);

        //System.Windows.Controls.MenuItem item4 = new System.Windows.Controls.MenuItem();
        //item4.Header = "TimeChecker"; item4.Click += MenuItemTimeChecker_Click; mainMenu.Items.Add(item4);

        mainMenu.Items.Add(new Separator());

        System.Windows.Controls.MenuItem item5 = new System.Windows.Controls.MenuItem();
        item5.Header = "Exit";
        item5.Click += MenuItemExit_Click;
        mainMenu.Items.Add(item5);

        fmMain.ContextMenu = mainMenu;
    }

    private void read_Config()
    {
        Left = Properties.Settings.Default.mainWndLeft;
        Top = Properties.Settings.Default.mainWndTop;
        isTransparent = Properties.Settings.Default.isTransparent;
        transparentValue = Properties.Settings.Default.transparentValue;
        ShowInTaskbar = Properties.Settings.Default.showInTaskBar;
        Topmost = Properties.Settings.Default.topMost;
        isAutoFold = Properties.Settings.Default.isAutoFold;
        useSnap = Properties.Settings.Default.useSnap;
    }

    private void save_Config()
    {
        Properties.Settings.Default.mainWndLeft = Left;
        Properties.Settings.Default.mainWndTop = Top;
        Properties.Settings.Default.isTransparent = isTransparent;
        Properties.Settings.Default.transparentValue = transparentValue;
        Properties.Settings.Default.showInTaskBar = ShowInTaskbar;
        Properties.Settings.Default.topMost = Topmost;
        Properties.Settings.Default.isAutoFold = isAutoFold;
        Properties.Settings.Default.useSnap = useSnap;

        Properties.Settings.Default.Save();
    }

    private void init_Timer()
    {
        timeTimer = new Timer();
        timeTimer.Interval = 1000;
        timeTimer.Tick += new EventHandler(ttTick);
        timeTimer.Start();
    }

    private void ttTick(object sender, EventArgs e)
    {
        if (newFmTimeChecker.Visibility == Visibility.Visible)
        {
            localTime = newFmTimeChecker.curTime.Date;
            localTime = localTime.AddMinutes(((int)newFmTimeChecker.slTimeChecker.Value) * 5);
            _TimeChecker2.rwTop.Height = new GridLength(_TimeChecker2.rwTop.MaxHeight *
                newFmTimeChecker.slTimeChecker.Value / newFmTimeChecker.slTimeChecker.Maximum);
        }
        else
            localTime = DateTime.Now;

        foreach (OsClock item in lstClock)
            item.setTime(localTime);
    }

    private void MenuItemSettings_Click(object sender, RoutedEventArgs e)
    {
        fmSettings newFmSettings = new fmSettings(this);
        newFmSettings.ShowDialog();
    }

    private void adjustTimeCheckerPosition()
    {
        _TimeChecker2.Left = Left;
        _TimeChecker2.Top = Top + Height;
        newFmTimeChecker.Left = this.Left;
        newFmTimeChecker.Top = this.Top + this.Height + _TimeChecker2.Height;
    }

    private void MenuItemTimeChecker_Click(object sender, RoutedEventArgs e)
    {
        if (this.newFmTimeChecker.Visibility == Visibility.Visible)
        {
            newFmTimeChecker.Hide();
            _TimeChecker2.Hide();
            timeTimer.Interval = 1000;
        }
        else
        {
            newFmTimeChecker.curTime = localTime;
            newFmTimeChecker.slTimeChecker.Value = localTime.Hour * 12 + (int)(localTime.Minute / 5);
            newFmTimeChecker.Show();
            _TimeChecker2.Show();
            timeTimer.Interval = 100;
        }

        fmMain_MouseLeave(null, null);
    }

    private void MenuItemExit_Click(object sender, RoutedEventArgs e)
    {
        System.Windows.Application.Current.Shutdown();
    }

    private void MenuItemAbout_Click(object sender, RoutedEventArgs e)
    {
        string version = null;
        Assembly assem = typeof(MainWindow).Assembly;
        version = Assembly.GetExecutingAssembly().GetName().Version.ToString();

        System.Windows.MessageBox.Show("Ozi clock. " + version);
    }

    private void fmMain_Closing(object sender, System.ComponentModel.CancelEventArgs e)
    {
        save_Config();
    }

    private void gdMain_MouseDown(object sender, MouseButtonEventArgs e)
    {
        e.Handled = true; //double click  

        if (e.ClickCount > 1) MenuItemTimeChecker_Click(null, null);

        if (e.ChangedButton == MouseButton.Left)
        {
            fmMain.DragMove();
        }

        if (e.ChangedButton == MouseButton.Middle)
            if (isFolded)
                UnFoldMainWindow();
            else
                FoldMainWindow();
    }

    private void foldingMainWindow()
    {
        int i = (int)this.Height;
        if (this.IsMouseOver)
        {
            while (i <= 62)
                if (this.IsMouseOver)
                {
                    Height = i++;
                    adjustTimeCheckerPosition();
                }
                else
                    return;
        }
        else
            while (i >= 29)
                if (!this.IsMouseOver)
                {
                    Height = i--;
                    adjustTimeCheckerPosition();
                }
                else
                    return;
    }

    private void fmMain_MouseEnter(object sender, System.Windows.Input.MouseEventArgs e)
    {
        if (isTransparent)
            this.Opacity = 1;
        if (isAutoFold && isFolded)
            foldingMainWindow();
    }

    private void fmMain_MouseLeave(object sender, System.Windows.Input.MouseEventArgs e)
    {
        if (isTransparent && !(newFmTimeChecker.Visibility == Visibility.Visible))
            this.Opacity = transparentValue;
        else
            this.Opacity = 100;
        if (isAutoFold && isFolded)
            foldingMainWindow();
    }

    private void FoldMainWindow()
    {
        {
            for (int i = (int)this.Height; i > 29; i--)
            {
                fmMain.Height = i;
                adjustTimeCheckerPosition();
            }

            isFolded = true;
        }
    }

    private void UnFoldMainWindow()
    {
        {
            for (int i = (int)this.Height; i < 62; i++)
            {
                fmMain.Height = i;
                adjustTimeCheckerPosition();
            }

            isFolded = false;
        }
    }

    private void fmMain_LocationChanged(object sender, EventArgs e)
    {
        if (useSnap)
            snapWindow();
        adjustTimeCheckerPosition();
    }

    private bool isAlreadySnapped = false;

    private void snapWindow()
    {
        if (!useSnap) return;

        // Screen screenTmp = null;
        // foreach (Screen screen in screens)
        //     if (screen.WorkingArea.Contains((int)(this.Left + this.Width / 2), (int)(this.Top + this.Height / 2)))
        //         screenTmp = screen;

        // if (screenTmp == null) return;
        //
        // if (Math.Abs(screenTmp.WorkingArea.Left - this.Left) <= 20)
        // {
        //     this.Left = screenTmp.WorkingArea.Left;
        // }
        //
        // int borderLim = 0;
        // if (!isAlreadySnapped)
        // {
        //     borderLim = 20;
        // }
        // else
        // {
        //     borderLim = 45;
        // }
        //
        // if (Math.Abs(screenTmp.WorkingArea.Top - this.Top) <= borderLim && !isAlreadySnapped)
        // {
        //     this.Top = screenTmp.WorkingArea.Top;
        //     isAlreadySnapped = true;
        // }
        // else
        // {
        //     isAlreadySnapped = false;
        // }
        //
        // if (Math.Abs(screenTmp.WorkingArea.Right - (this.Left + this.Width)) <= 20)
        // {
        //     this.Left = screenTmp.WorkingArea.Right - this.Width;
        // }
        //
        // if (Math.Abs(screenTmp.WorkingArea.Bottom - (this.Top + this.Height)) <= 20)
        // {
        //     this.Top = screenTmp.WorkingArea.Bottom - this.Height;
        // }
    }
}